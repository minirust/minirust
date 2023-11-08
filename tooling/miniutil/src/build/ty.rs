use crate::build::*;

pub fn layout(size: Size, align: Align) -> Layout {
    Layout {
        size,
        align,
        inhabited: true, // currently everything is inhabited (enums don't exist yet).
    }
}

pub fn int_ty(signed: Signedness, size: Size) -> Type {
    Type::Int(IntType { signed, size })
}

pub fn bool_ty() -> Type {
    Type::Bool
}

pub fn ref_ty(pointee: Layout) -> Type {
    Type::Ptr(PtrType::Ref {
        mutbl: Mutability::Immutable,
        pointee,
    })
}

pub fn ref_mut_ty(pointee: Layout) -> Type {
    Type::Ptr(PtrType::Ref {
        mutbl: Mutability::Mutable,
        pointee,
    })
}

pub fn box_ty(pointee: Layout) -> Type {
    Type::Ptr(PtrType::Box { pointee })
}

pub fn raw_ptr_ty() -> Type {
    Type::Ptr(PtrType::Raw)
}

pub fn tuple_ty(f: &[(Offset, Type)], size: Size, align: Align) -> Type {
    Type::Tuple {
        fields: f.iter().copied().collect(),
        size,
        align,
    }
}

pub fn union_ty(f: &[(Offset, Type)], size: Size, align: Align) -> Type {
    let chunks = list![(Size::ZERO, size)];
    Type::Union {
        fields: f.iter().copied().collect(),
        size,
        align,
        chunks,
    }
}

pub fn array_ty(elem: Type, count: impl Into<Int>) -> Type {
    Type::Array {
        elem: GcCow::new(elem),
        count: count.into(),
    }
}

pub fn enum_variant(ty: Type, tagger: &[(Offset, u8)]) -> Variant {
    Variant {
        ty,
        tagger: tagger.iter().copied().collect()
    }
}

pub fn enum_ty(variants: &[Variant], discriminator: Discriminator, size: Size, align: Align) -> Type {
    Type::Enum {
        variants: variants.iter().copied().collect(),
        discriminator,
        size,
        align,
    }
}
