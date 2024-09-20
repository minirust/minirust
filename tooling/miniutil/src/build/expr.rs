use crate::build::*;

// Example usage:
// `const_int(42usize)`
pub fn const_int<T: TypeConv + Into<Int>>(int: T) -> ValueExpr {
    ValueExpr::Constant(Constant::Int(int.into()), T::get_type())
}
pub fn const_int_typed<T: TypeConv>(int: Int) -> ValueExpr {
    ValueExpr::Constant(Constant::Int(int), T::get_type())
}

pub fn const_bool(b: bool) -> ValueExpr {
    ValueExpr::Constant(Constant::Bool(b), Type::Bool)
}

#[track_caller]
pub fn tuple(args: &[ValueExpr], ty: Type) -> ValueExpr {
    let Type::Tuple { fields, .. } = ty else {
        panic!("const_tuple received non-tuple type!");
    };
    assert_eq!(fields.len(), args.len());
    ValueExpr::Tuple(args.iter().cloned().collect(), ty)
}

pub fn array(args: &[ValueExpr], elem_ty: Type) -> ValueExpr {
    let ty = array_ty(elem_ty, args.len());
    ValueExpr::Tuple(args.iter().cloned().collect(), ty)
}

pub fn variant(discriminant: impl Into<Int>, data: ValueExpr, enum_ty: Type) -> ValueExpr {
    ValueExpr::Variant { discriminant: discriminant.into(), data: GcCow::new(data), enum_ty }
}

pub fn get_discriminant(place: PlaceExpr) -> ValueExpr {
    ValueExpr::GetDiscriminant { place: GcCow::new(place) }
}

// Returns () or [].
pub fn unit() -> ValueExpr {
    ValueExpr::Tuple(Default::default(), <()>::get_type())
}

pub fn null() -> ValueExpr {
    ValueExpr::Constant(Constant::PointerWithoutProvenance(0.into()), <*const ()>::get_type())
}

pub fn load(p: PlaceExpr) -> ValueExpr {
    ValueExpr::Load { source: GcCow::new(p) }
}

#[track_caller]
pub fn addr_of(target: PlaceExpr, ty: Type) -> ValueExpr {
    let Type::Ptr(ptr_ty) = ty else {
        panic!("addr_of requires a Type::Ptr!");
    };
    ValueExpr::AddrOf { target: GcCow::new(target), ptr_ty }
}

/// Unary `-` on an integer.
pub fn neg(v: ValueExpr) -> ValueExpr {
    ValueExpr::UnOp { operator: UnOp::Int(IntUnOp::Neg), operand: GcCow::new(v) }
}

/// Unary `!` on an integer
pub fn bit_not(v: ValueExpr) -> ValueExpr {
    ValueExpr::UnOp { operator: UnOp::Int(IntUnOp::BitNot), operand: GcCow::new(v) }
}

#[track_caller]
pub fn int_cast<T: TypeConv>(v: ValueExpr) -> ValueExpr {
    let Type::Int(t) = T::get_type() else {
        panic!("int operator received non-int type!");
    };
    ValueExpr::UnOp { operator: UnOp::Cast(CastOp::IntToInt(t)), operand: GcCow::new(v) }
}

pub fn ptr_addr(v: ValueExpr) -> ValueExpr {
    transmute(v, <usize>::get_type())
}

#[track_caller]
pub fn ptr_to_ptr(v: ValueExpr, t: Type) -> ValueExpr {
    let Type::Ptr(_) = t else {
        panic!("ptr_to_ptr requires Type::Ptr argument!");
    };
    transmute(v, t)
}

pub fn bool_to_int<T: TypeConv>(v: ValueExpr) -> ValueExpr {
    // First transmute to `u8`.
    let t_u8 = u8::get_type();
    let int = transmute(v, t_u8);
    // Then cast that to the desired integer type, if necessary.
    if T::get_type() == t_u8 { int } else { int_cast::<T>(int) }
}

pub fn not(v: ValueExpr) -> ValueExpr {
    // `1 - v` is always 0 or 1, so safe to transmute back
    transmute(sub(const_int(1u8), bool_to_int::<u8>(v)), bool_ty())
}

pub fn transmute(v: ValueExpr, t: Type) -> ValueExpr {
    ValueExpr::UnOp { operator: UnOp::Cast(CastOp::Transmute(t)), operand: GcCow::new(v) }
}

pub fn get_thin_pointer(v: ValueExpr) -> ValueExpr {
    ValueExpr::UnOp { operator: UnOp::GetThinPointer, operand: GcCow::new(v) }
}

pub fn get_metadata(v: ValueExpr) -> ValueExpr {
    ValueExpr::UnOp { operator: UnOp::GetMetadata, operand: GcCow::new(v) }
}

pub fn construct_wide_pointer(ptr: ValueExpr, meta: ValueExpr, ptr_ty: Type) -> ValueExpr {
    let Type::Ptr(ptr_ty) = ptr_ty else {
        panic!("construct_wide_pointer requires Type::Ptr argument!");
    };

    ValueExpr::BinOp {
        operator: BinOp::ConstructWidePointer(ptr_ty),
        left: GcCow::new(ptr),
        right: GcCow::new(meta),
    }
}

fn int_binop(op: IntBinOp, l: ValueExpr, r: ValueExpr) -> ValueExpr {
    ValueExpr::BinOp { operator: BinOp::Int(op), left: GcCow::new(l), right: GcCow::new(r) }
}

pub fn add(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::Add, l, r)
}
pub fn add_unchecked(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::AddUnchecked, l, r)
}
pub fn sub(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::Sub, l, r)
}
pub fn sub_unchecked(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::SubUnchecked, l, r)
}
pub fn mul(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::Mul, l, r)
}
pub fn mul_unchecked(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::MulUnchecked, l, r)
}
pub fn div(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::Div, l, r)
}
pub fn div_exact(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::DivExact, l, r)
}
pub fn rem(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::Rem, l, r)
}
pub fn shl(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::Shl, l, r)
}
pub fn shl_unchecked(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::ShlUnchecked, l, r)
}
pub fn shr(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::Shr, l, r)
}
pub fn shr_unchecked(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::ShrUnchecked, l, r)
}
pub fn bit_and(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::BitAnd, l, r)
}
pub fn bit_or(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::BitOr, l, r)
}
pub fn bit_xor(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::BitXor, l, r)
}

fn int_overflow(op: IntBinOpWithOverflow, l: ValueExpr, r: ValueExpr) -> ValueExpr {
    ValueExpr::BinOp {
        operator: BinOp::IntWithOverflow(op),
        left: GcCow::new(l),
        right: GcCow::new(r),
    }
}

pub fn overflow_add(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_overflow(IntBinOpWithOverflow::Add, l, r)
}
pub fn overflow_sub(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_overflow(IntBinOpWithOverflow::Sub, l, r)
}
pub fn overflow_mul(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_overflow(IntBinOpWithOverflow::Mul, l, r)
}

fn rel_op(op: RelOp, l: ValueExpr, r: ValueExpr) -> ValueExpr {
    ValueExpr::BinOp { operator: BinOp::Rel(op), left: GcCow::new(l), right: GcCow::new(r) }
}

pub fn eq(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    rel_op(RelOp::Eq, l, r)
}

pub fn ne(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    rel_op(RelOp::Ne, l, r)
}

pub fn ge(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    rel_op(RelOp::Ge, l, r)
}

pub fn gt(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    rel_op(RelOp::Gt, l, r)
}

pub fn le(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    rel_op(RelOp::Le, l, r)
}

pub fn lt(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    rel_op(RelOp::Lt, l, r)
}
pub fn cmp(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    ValueExpr::BinOp { operator: BinOp::Rel(RelOp::Cmp), left: GcCow::new(l), right: GcCow::new(r) }
}

pub fn bool_and(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    // `l & r` is always 0 or 1, so safe to transmute back
    transmute(bit_and(bool_to_int::<u8>(l), bool_to_int::<u8>(r)), bool_ty())
}
pub fn bool_or(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    // `l | r` is always 0 or 1, so safe to transmute back
    transmute(bit_or(bool_to_int::<u8>(l), bool_to_int::<u8>(r)), bool_ty())
}
pub fn bool_xor(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    // `l ^ r` is always 0 or 1, so safe to transmute back
    transmute(bit_xor(bool_to_int::<u8>(l), bool_to_int::<u8>(r)), bool_ty())
}

pub enum InBounds {
    Yes,
    No,
}

pub fn ptr_offset(l: ValueExpr, r: ValueExpr, inbounds: InBounds) -> ValueExpr {
    ValueExpr::BinOp {
        operator: BinOp::PtrOffset { inbounds: matches!(inbounds, InBounds::Yes) },
        left: GcCow::new(l),
        right: GcCow::new(r),
    }
}

pub fn ptr_offset_from(l: ValueExpr, r: ValueExpr, inbounds: InBounds) -> ValueExpr {
    ValueExpr::BinOp {
        operator: BinOp::PtrOffsetFrom {
            inbounds: matches!(inbounds, InBounds::Yes),
            nonneg: false,
        },
        left: GcCow::new(l),
        right: GcCow::new(r),
    }
}

pub fn ptr_offset_from_nonneg(l: ValueExpr, r: ValueExpr, inbounds: InBounds) -> ValueExpr {
    ValueExpr::BinOp {
        operator: BinOp::PtrOffsetFrom {
            inbounds: matches!(inbounds, InBounds::Yes),
            nonneg: true,
        },
        left: GcCow::new(l),
        right: GcCow::new(r),
    }
}

pub fn local_by_name(name: LocalName) -> PlaceExpr {
    PlaceExpr::Local(name)
}

pub fn local(x: u32) -> PlaceExpr {
    local_by_name(LocalName(Name::from_internal(x)))
}

pub fn global_by_name<T: TypeConv>(name: GlobalName) -> PlaceExpr {
    let relocation = Relocation { name, offset: Size::ZERO };
    let ptr_type = Type::Ptr(PtrType::Raw { meta_kind: PointerMetaKind::None });
    deref(ValueExpr::Constant(Constant::GlobalPointer(relocation), ptr_type), T::get_type())
}

pub fn global<T: TypeConv>(x: u32) -> PlaceExpr {
    global_by_name::<T>(GlobalName(Name::from_internal(x)))
}

pub fn deref(operand: ValueExpr, ty: Type) -> PlaceExpr {
    PlaceExpr::Deref { operand: GcCow::new(operand), ty }
}

pub fn field(root: PlaceExpr, field: impl Into<Int>) -> PlaceExpr {
    PlaceExpr::Field { root: GcCow::new(root), field: field.into() }
}

pub fn index(root: PlaceExpr, index: ValueExpr) -> PlaceExpr {
    PlaceExpr::Index { root: GcCow::new(root), index: GcCow::new(index) }
}

/// An enum downcast into the variant at the specified index.
pub fn downcast(root: PlaceExpr, discriminant: impl Into<Int>) -> PlaceExpr {
    PlaceExpr::Downcast { root: GcCow::new(root), discriminant: discriminant.into() }
}

/// A place suited for 1-aligned zero-sized accesses.
pub fn unit_place() -> PlaceExpr {
    let ptr =
        ValueExpr::Constant(Constant::PointerWithoutProvenance(1.into()), <*const ()>::get_type());
    PlaceExpr::Deref { operand: GcCow::new(ptr), ty: <()>::get_type() }
}

pub fn by_value(val: ValueExpr) -> ArgumentExpr {
    ArgumentExpr::ByValue(val)
}

pub fn in_place(arg: PlaceExpr) -> ArgumentExpr {
    ArgumentExpr::InPlace(arg)
}
