use crate::build::*;

pub fn const_int<T: TypeConv>(int: impl Into<Int>) -> ValueExpr {
    ValueExpr::Constant(Constant::Int(int.into()), T::get_type())
}

pub fn const_bool(b: bool) -> ValueExpr {
    ValueExpr::Constant(Constant::Bool(b), Type::Bool)
}

// this gets ValueExprs instead of Constants to be compatible with the functions above.
pub fn const_tuple(args: &[ValueExpr], ty: Type) -> ValueExpr {
    let Type::Tuple { fields, .. } = ty else {
        panic!("const_tuple received non-tuple type!");
    };
    assert_eq!(fields.len(), args.len());
    ValueExpr::Tuple(args.iter().cloned().collect(), ty)
}

// doesn't support zero-length arrays, as their type wouldn't be clear.
pub fn const_array(args: &[ValueExpr], elem_ty: Type) -> ValueExpr {
    let ty = array_ty(elem_ty, args.len());
    ValueExpr::Tuple(args.iter().cloned().collect(), ty)
}

// returns () or [].
pub fn const_unit() -> ValueExpr {
    ValueExpr::Tuple(Default::default(), <()>::get_type())
}

// non-destructive load.
pub fn load(p: PlaceExpr) -> ValueExpr {
    ValueExpr::Load {
        source: GcCow::new(p),
        destructive: false,
    }
}

pub fn load_destructive(p: PlaceExpr) -> ValueExpr {
    ValueExpr::Load {
        source: GcCow::new(p),
        destructive: true,
    }
}

pub fn addr_of(target: PlaceExpr, ty: Type) -> ValueExpr {
    let Type::Ptr(ptr_ty) = ty else {
        panic!("addr_of requires a Type::Ptr!");
    };
    ValueExpr::AddrOf {
        target: GcCow::new(target),
        ptr_ty,
    }
}

pub fn neg<T: TypeConv>(v: ValueExpr) -> ValueExpr {
    let Type::Int(t) = T::get_type() else {
        panic!("int operator received non-int type!");
    };
    ValueExpr::UnOp {
        operator: UnOp::Int(UnOpInt::Neg, t),
        operand: GcCow::new(v),
    }
}

pub fn int_cast<T: TypeConv>(v: ValueExpr) -> ValueExpr {
    let Type::Int(t) = T::get_type() else {
        panic!("int operator received non-int type!");
    };
    ValueExpr::UnOp {
        operator: UnOp::Int(UnOpInt::Cast, t),
        operand: GcCow::new(v),
    }
}

pub fn ptr_to_int(v: ValueExpr) -> ValueExpr {
    ValueExpr::UnOp {
        operator: UnOp::Ptr2Int,
        operand: GcCow::new(v),
    }
}

pub fn int_to_ptr(v: ValueExpr, t: Type) -> ValueExpr {
    let Type::Ptr(ptr_ty) = t else {
        panic!("int_to_ptr requires Type::Ptr argument!");
    };
    ValueExpr::UnOp {
        operator: UnOp::Int2Ptr(ptr_ty),
        operand: GcCow::new(v),
    }
}

fn int_binop<T: TypeConv>(op: BinOpInt, l: ValueExpr, r: ValueExpr) -> ValueExpr {
    let Type::Int(t) = T::get_type() else {
        panic!("int operator received non-int type!");
    };
    ValueExpr::BinOp {
        operator: BinOp::Int(op, t),
        left: GcCow::new(l),
        right: GcCow::new(r),
    }
}

pub fn add<T: TypeConv>(l: ValueExpr, r: ValueExpr) -> ValueExpr { int_binop::<T>(BinOpInt::Add, l, r) }
pub fn sub<T: TypeConv>(l: ValueExpr, r: ValueExpr) -> ValueExpr { int_binop::<T>(BinOpInt::Sub, l, r) }
pub fn mul<T: TypeConv>(l: ValueExpr, r: ValueExpr) -> ValueExpr { int_binop::<T>(BinOpInt::Mul, l, r) }
pub fn div<T: TypeConv>(l: ValueExpr, r: ValueExpr) -> ValueExpr { int_binop::<T>(BinOpInt::Div, l, r) }

pub enum InBounds { Yes, No}

pub fn ptr_offset(l: ValueExpr, r: ValueExpr, inbounds: InBounds) -> ValueExpr {
    ValueExpr::BinOp {
        operator: BinOp::PtrOffset { inbounds: matches!(inbounds, InBounds::Yes) },
        left: GcCow::new(l),
        right: GcCow::new(r),
    }
}

pub fn local(x: u32) -> PlaceExpr {
    PlaceExpr::Local(LocalName(Name::new(x)))
}

pub fn deref(operand: ValueExpr, ptype: PlaceType) -> PlaceExpr {
    PlaceExpr::Deref {
        operand: GcCow::new(operand),
        ptype,
    }
}

pub fn field(root: PlaceExpr, field: impl Into<Int>) -> PlaceExpr {
    PlaceExpr::Field {
        root: GcCow::new(root),
        field: field.into(),
    }
}

pub fn index(root: PlaceExpr, index: ValueExpr) -> PlaceExpr {
    PlaceExpr::Index {
        root: GcCow::new(root),
        index: GcCow::new(index),
    }
}
