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
pub fn int_neg(v: ValueExpr) -> ValueExpr {
    ValueExpr::UnOp { operator: UnOp::Int(IntUnOp::Neg), operand: GcCow::new(v) }
}

/// Unary `!` on an integer
pub fn int_not(v: ValueExpr) -> ValueExpr {
    ValueExpr::UnOp { operator: UnOp::Int(IntUnOp::Not), operand: GcCow::new(v) }
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

#[track_caller]
pub fn bool_to_int<T: TypeConv + Into<Int>>(v: ValueExpr) -> ValueExpr {
    let Type::Int(int_ty) = T::get_type() else {
        panic!("bool_to_int needs <T> to be converted to Type::Int!");
    };
    ValueExpr::UnOp { operator: UnOp::Cast(CastOp::BoolToInt(int_ty)), operand: GcCow::new(v) }
}

pub fn not(v: ValueExpr) -> ValueExpr {
    ValueExpr::UnOp { operator: UnOp::Bool(BoolUnOp::Not), operand: GcCow::new(v) }
}

pub fn transmute(v: ValueExpr, t: Type) -> ValueExpr {
    ValueExpr::UnOp { operator: UnOp::Cast(CastOp::Transmute(t)), operand: GcCow::new(v) }
}

fn int_binop(op: IntBinOp, l: ValueExpr, r: ValueExpr) -> ValueExpr {
    ValueExpr::BinOp { operator: BinOp::Int(op), left: GcCow::new(l), right: GcCow::new(r) }
}

pub fn add(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::Add, l, r)
}
pub fn unchecked_add(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::AddUnchecked, l, r)
}
pub fn sub(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::Sub, l, r)
}
pub fn unchecked_sub(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::SubUnchecked, l, r)
}
pub fn mul(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::Mul, l, r)
}
pub fn unchecked_mul(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::MulUnchecked, l, r)
}
pub fn div(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::Div, l, r)
}
pub fn shl(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::Shl, l, r)
}
pub fn unchecked_shl(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::ShlUnchecked, l, r)
}
pub fn shr(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(IntBinOp::Shr, l, r)
}
pub fn unchecked_shr(l: ValueExpr, r: ValueExpr) -> ValueExpr {
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

fn int_rel(op: IntRel, l: ValueExpr, r: ValueExpr) -> ValueExpr {
    ValueExpr::BinOp { operator: BinOp::IntRel(op), left: GcCow::new(l), right: GcCow::new(r) }
}

pub fn eq(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_rel(IntRel::Eq, l, r)
}

pub fn ne(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_rel(IntRel::Ne, l, r)
}

pub fn ge(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_rel(IntRel::Ge, l, r)
}

pub fn gt(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_rel(IntRel::Gt, l, r)
}

pub fn le(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_rel(IntRel::Le, l, r)
}

pub fn lt(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_rel(IntRel::Lt, l, r)
}
pub fn cmp(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    ValueExpr::BinOp { operator: BinOp::Cmp, left: GcCow::new(l), right: GcCow::new(r) }
}

fn bool_binop(op: BoolBinOp, l: ValueExpr, r: ValueExpr) -> ValueExpr {
    ValueExpr::BinOp { operator: BinOp::Bool(op), left: GcCow::new(l), right: GcCow::new(r) }
}
pub fn bool_and(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    bool_binop(BoolBinOp::BitAnd, l, r)
}
pub fn bool_or(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    bool_binop(BoolBinOp::BitOr, l, r)
}
pub fn bool_xor(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    bool_binop(BoolBinOp::BitXor, l, r)
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

pub fn local_by_name(name: LocalName) -> PlaceExpr {
    PlaceExpr::Local(name)
}

pub fn local(x: u32) -> PlaceExpr {
    local_by_name(LocalName(Name::from_internal(x)))
}

pub fn global_by_name<T: TypeConv>(name: GlobalName) -> PlaceExpr {
    let relocation = Relocation { name, offset: Size::ZERO };
    let ptr_type = Type::Ptr(PtrType::Raw);
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
pub fn zst_place() -> PlaceExpr {
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
