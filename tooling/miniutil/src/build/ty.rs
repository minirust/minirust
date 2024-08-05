use crate::build::*;

pub fn layout(size: SizeStrategy, align: Align) -> Layout {
    Layout {
        size,
        align,
        // FIXME: enums do exist, is this still good?
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
    Type::Ptr(PtrType::Ref { mutbl: Mutability::Immutable, pointee })
}

pub fn ref_mut_ty(pointee: Layout) -> Type {
    Type::Ptr(PtrType::Ref { mutbl: Mutability::Mutable, pointee })
}

pub fn box_ty(pointee: Layout) -> Type {
    Type::Ptr(PtrType::Box { pointee })
}

pub fn raw_ptr_ty(pointee: Layout) -> Type {
    Type::Ptr(PtrType::Raw { pointee })
}

pub fn raw_void_ptr_ty() -> Type {
    let pointee = layout(SizeStrategy::Sized(Size::ZERO), Align::ONE);
    raw_ptr_ty(pointee)
}

/// A type `(*mut T, usize)` that is compatible with `*mut [T]`.
pub fn slice_ptr_tuple_ty<T: TypeConv>() -> Type {
    assert_eq!(<usize>::get_size().unwrap_size().bytes(), 8, "Assumes 8 byte pointers");
    tuple_ty(&[(size(0), <*mut T>::get_type()), (size(8), <usize>::get_type())], size(16), align(1))
}

pub fn tuple_ty(f: &[(Offset, Type)], size: Size, align: Align) -> Type {
    Type::Tuple { fields: f.iter().copied().collect(), size, align }
}

pub fn union_ty(f: &[(Offset, Type)], size: Size, align: Align) -> Type {
    let chunks = list![(Size::ZERO, size)];
    Type::Union { fields: f.iter().copied().collect(), size, align, chunks }
}

pub fn array_ty(elem: Type, count: impl Into<Int>) -> Type {
    Type::Array { elem: GcCow::new(elem), count: count.into() }
}

pub fn slice_ty(elem: Type) -> Type {
    Type::Slice { elem: GcCow::new(elem) }
}

pub fn enum_variant(ty: Type, tagger: &[(Offset, (IntType, Int))]) -> Variant {
    Variant { ty, tagger: tagger.iter().copied().collect() }
}

pub fn enum_ty<DiscriminantTy: TypeConv + Into<Int> + Copy>(
    variants: &[(DiscriminantTy, Variant)],
    discriminator: Discriminator,
    size: Size,
    align: Align,
) -> Type {
    let Type::Int(discriminant_ty) = DiscriminantTy::get_type() else {
        panic!("Discriminant Type needs to be an integer type.");
    };
    Type::Enum {
        variants: variants.iter().copied().map(|(disc, variant)| (disc.into(), variant)).collect(),
        discriminator,
        discriminant_ty,
        size,
        align,
    }
}

pub fn discriminator_invalid() -> Discriminator {
    Discriminator::Invalid
}

pub fn discriminator_known(discriminant: impl Into<Int>) -> Discriminator {
    Discriminator::Known(discriminant.into())
}

/// Builds a branching discriminator on the type given by the generic which has to be an integer type.
pub fn discriminator_branch<T: ToInt + TypeConv + Copy>(
    offset: Offset,
    fallback: Discriminator,
    children: &[((T, T), Discriminator)],
) -> Discriminator {
    let Type::Int(value_type) = T::get_type() else { unreachable!() };
    Discriminator::Branch {
        offset,
        value_type,
        fallback: GcCow::new(fallback),
        children: children
            .into_iter()
            .copied()
            .map(|((start, end), disc)| ((start.into(), end.into()), disc))
            .collect(),
    }
}
