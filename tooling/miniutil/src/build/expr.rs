use crate::build::*;

// Example usage:
// `const_int::<usize>(42)`
pub fn const_int<T: TypeConv>(int: impl Into<Int>) -> ValueExpr {
    ValueExpr::Constant(Constant::Int(int.into()), T::get_type())
}

pub fn const_bool(b: bool) -> ValueExpr {
    ValueExpr::Constant(Constant::Bool(b), Type::Bool)
}

pub fn const_tuple(args: &[ValueExpr], ty: Type) -> ValueExpr {
    let Type::Tuple { fields, .. } = ty else {
        panic!("const_tuple received non-tuple type!");
    };
    assert_eq!(fields.len(), args.len());
    ValueExpr::Tuple(args.iter().cloned().collect(), ty)
}

pub fn const_array(args: &[ValueExpr], elem_ty: Type) -> ValueExpr {
    let ty = array_ty(elem_ty, args.len());
    ValueExpr::Tuple(args.iter().cloned().collect(), ty)
}

// Returns () or [].
pub fn const_unit() -> ValueExpr {
    ValueExpr::Tuple(Default::default(), <()>::get_type())
}

pub fn load(p: PlaceExpr) -> ValueExpr {
    ValueExpr::Load {
        source: GcCow::new(p),
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

// Example usage:
// `neg::<i32>(42)`
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

pub fn int_to_ptr(v: ValueExpr, t: Type) -> ValueExpr {
    let Type::Ptr(ptr_ty) = t else {
        panic!("int_to_ptr requires Type::Ptr argument!");
    };
    ValueExpr::UnOp {
        operator: UnOp::PtrFromExposed(ptr_ty),
        operand: GcCow::new(v),
    }
}

pub fn ptr_addr(v: ValueExpr) -> ValueExpr {
    ValueExpr::UnOp {
        operator: UnOp::PtrAddr,
        operand: GcCow::new(v),
    }
}

pub fn ptr_to_ptr(v: ValueExpr, t: Type) -> ValueExpr {
    let Type::Ptr(ptr_ty) = t else {
        panic!("ptr_to_ptr requires Type::Ptr argument!");
    };
    ValueExpr::UnOp {
        operator: UnOp::PtrCast(ptr_ty),
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

pub fn add<T: TypeConv>(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop::<T>(BinOpInt::Add, l, r)
}
pub fn sub<T: TypeConv>(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop::<T>(BinOpInt::Sub, l, r)
}
pub fn mul<T: TypeConv>(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop::<T>(BinOpInt::Mul, l, r)
}
pub fn div<T: TypeConv>(l: ValueExpr, r: ValueExpr) -> ValueExpr {
    int_binop::<T>(BinOpInt::Div, l, r)
}

fn int_rel(op: IntRel, l: ValueExpr, r: ValueExpr) -> ValueExpr {
    ValueExpr::BinOp {
        operator: BinOp::IntRel(op),
        left: GcCow::new(l),
        right: GcCow::new(r),
    }
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

pub enum InBounds {
    Yes,
    No,
}

pub fn ptr_offset(l: ValueExpr, r: ValueExpr, inbounds: InBounds) -> ValueExpr {
    ValueExpr::BinOp {
        operator: BinOp::PtrOffset {
            inbounds: matches!(inbounds, InBounds::Yes),
        },
        left: GcCow::new(l),
        right: GcCow::new(r),
    }
}

pub fn local(x: u32) -> PlaceExpr {
    PlaceExpr::Local(LocalName(Name::from_internal(x)))
}

pub fn global<T: TypeConv>(x: u32) -> PlaceExpr {
    let relocation = Relocation {
        name: GlobalName(Name::from_internal(x)),
        offset: Size::ZERO
    };

    let ptr_type = Type::Ptr(PtrType::Raw);

    deref(
        ValueExpr::Constant(Constant::GlobalPointer(relocation), ptr_type),
        T::get_ptype()
    )
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
