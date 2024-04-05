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

pub fn addr_of(target: PlaceExpr, ty: Type) -> ValueExpr {
    let Type::Ptr(ptr_ty) = ty else {
        panic!("addr_of requires a Type::Ptr!");
    };
    ValueExpr::AddrOf { target: GcCow::new(target), ptr_ty }
}

/// Unary `-` on an integer.
pub fn int_neg(v: ValueExpr) -> ValueExpr {
    ValueExpr::UnOp { operator: UnOp::Int(UnOpInt::Neg), operand: GcCow::new(v) }
}

pub fn int_cast<T: TypeConv>(v: ValueExpr) -> ValueExpr {
    let Type::Int(t) = T::get_type() else {
        panic!("int operator received non-int type!");
    };
    ValueExpr::UnOp { operator: UnOp::Cast(CastOp::IntToInt(t)), operand: GcCow::new(v) }
}

pub fn int_to_ptr(v: ValueExpr, t: Type) -> ValueExpr {
    let Type::Ptr(ptr_ty) = t else {
        panic!("int_to_ptr requires Type::Ptr argument!");
    };
    ValueExpr::UnOp { operator: UnOp::Cast(CastOp::PtrFromExposed(ptr_ty)), operand: GcCow::new(v) }
}

pub fn ptr_addr(v: ValueExpr) -> ValueExpr {
    transmute(v, <usize>::get_type())
}

pub fn ptr_to_ptr(v: ValueExpr, t: Type) -> ValueExpr {
    let Type::Ptr(_) = t else {
        panic!("ptr_to_ptr requires Type::Ptr argument!");
    };
    transmute(v, t)
}

pub fn bool_to_int<T: TypeConv + Into<Int>>(v: ValueExpr) -> ValueExpr {
    let Type::Int(int_ty) = T::get_type() else {
        panic!("bool_to_int needs <T> to be converted to Type::Int!");
    };
    ValueExpr::UnOp { operator: UnOp::Cast(CastOp::BoolToInt(int_ty)), operand: GcCow::new(v) }
}

pub fn not(v: ValueExpr) -> ValueExpr {
    ValueExpr::UnOp { operator: UnOp::Bool(UnOpBool::Not), operand: GcCow::new(v) }
}

pub fn transmute(v: ValueExpr, t: Type) -> ValueExpr {
    ValueExpr::UnOp { operator: UnOp::Cast(CastOp::Transmute(t)), operand: GcCow::new(v) }
}

fn int_binop(op: BinOpInt, l: ValueExpr, r: ValueExpr) -> ValueExpr {
    ValueExpr::BinOp { operator: BinOp::Int(op), left: GcCow::new(l), right: GcCow::new(r) }
}

pub fn add(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(BinOpInt::Add, l, r)
}
pub fn sub(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(BinOpInt::Sub, l, r)
}
pub fn mul(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(BinOpInt::Mul, l, r)
}
pub fn div(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(BinOpInt::Div, l, r)
}
pub fn bit_and(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop(BinOpInt::BitAnd, l, r)
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

fn bool_binop(op: BinOpBool, l: ValueExpr, r: ValueExpr) -> ValueExpr {
    ValueExpr::BinOp { operator: BinOp::Bool(op), left: GcCow::new(l), right: GcCow::new(r) }
}
pub fn bool_and(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    bool_binop(BinOpBool::BitAnd, l, r)
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

pub fn local(x: u32) -> PlaceExpr {
    PlaceExpr::Local(LocalName(Name::from_internal(x)))
}

pub fn global<T: TypeConv>(x: u32) -> PlaceExpr {
    let relocation = Relocation { name: GlobalName(Name::from_internal(x)), offset: Size::ZERO };

    let ptr_type = Type::Ptr(PtrType::Raw);

    deref(ValueExpr::Constant(Constant::GlobalPointer(relocation), ptr_type), T::get_type())
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
