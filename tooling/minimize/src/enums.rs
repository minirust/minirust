use crate::*;
mod rs {
    pub use crate::rs::*;
    pub use crate::rustc_target::abi::{Variants, FieldsShape, Primitive, TagEncoding, VariantIdx};
}

pub fn translate_enum<'tcx>(
    ty: rs::Ty<'tcx>,
    adt_def: rs::AdtDef<'tcx>,
    sref: rs::GenericArgsRef<'tcx>,
    tcx: rs::TyCtxt<'tcx>,
) -> Type {
    let a = rs::ParamEnv::empty().and(ty);
    let layout = tcx.layout_of(a).unwrap().layout;
    let size = translate_size(layout.size());
    let align = translate_align(layout.align().abi);

    let Type::Int(discriminant_ty) = translate_ty(ty.discriminant_ty(tcx), tcx) else {
        panic!("Discriminant type is not integer!")
    };

    let (variants, discriminator) = match layout.variants() {
        rs::Variants::Single { index } => {
            let fields = translate_fields(layout.fields(), adt_def.variant(*index), sref, tcx);
            ([(Int::ZERO, Variant { ty: Type::Tuple { fields, size, align }, tagger: Map::new() })].into_iter().collect(), Discriminator::Known(0.into()))
        },
        rs::Variants::Multiple {
            tag,
            tag_encoding,
            tag_field,
            variants,
        } => {

            // compute the offset of the tag for the tagger and discriminator construction
            let tag_offset: Offset = translate_size(layout.fields().offset(*tag_field));
            let tag_ty = match tag.primitive() {
                rs::Primitive::Int(ity, signed) => IntType { signed: if signed { Signedness::Signed } else { Signedness::Unsigned }, size: translate_size(ity.size()) },
                _ => panic!("enum tag has invalid primitive type"),
            };

            // translate the variants
            let mut translated_variants = Map::new();
            let mut discriminator_children = Map::new();
            for (variant_idx, variant_def) in adt_def.variants().iter_enumerated() {
                let fields = translate_fields(&variants[variant_idx].fields, &variant_def, sref, tcx);
                let discr = adt_def.discriminant_for_variant(tcx, variant_idx);
                let discr_int = int_from_bits(discr.val, discriminant_ty);
                let (tagger, tag) = match tag_encoding {
                    rs::TagEncoding::Direct => (
                        // direct tagging places the discriminant in the tag for all variants
                        [(tag_offset, (tag_ty, discr_int))].into_iter().collect(),
                        discr_int
                    ),
                    rs::TagEncoding::Niche { .. } => todo!("Implement Niche-encoded tags for enums (Timon)"),
                };
                translated_variants.insert(discr_int, Variant { ty: Type::Tuple { fields, size, align }, tagger });
                discriminator_children.insert(tag, Discriminator::Known(discr_int));
            }

            // build the discriminator.
            let discriminator = match tag_encoding {
                rs::TagEncoding::Direct =>
                    Discriminator::Branch { offset: tag_offset, value_type: tag_ty, fallback: GcCow::new(Discriminator::Invalid), children: discriminator_children },
                rs::TagEncoding::Niche { .. } => todo!("Implement Niche-encoded tags for enums (Timon)"),
            };
            (translated_variants, discriminator)
        },
    };


    Type::Enum {
        variants,
        discriminator,
        discriminant_ty,
        size,
        align,
    }
}


/// Constructs the fields of a given variant.
fn translate_fields<'tcx>(
    shape: &rs::FieldsShape,
    variant: &rs::VariantDef,
    sref: rs::GenericArgsRef<'tcx>,
    tcx: rs::TyCtxt<'tcx>,
) -> List<(Offset, Type)> {
    variant.fields
           .iter_enumerated()
           .map(|(i, field)| {
                let ty = field.ty(tcx, sref);
                let ty = translate_ty(ty, tcx);
                let offset = shape.offset(i.into());
                let offset = translate_size(offset);

                (offset, ty)
    }).collect()
}

pub fn int_from_bits(bits: u128, ity: IntType) -> Int {
    let n_bits = ity.size.bits().try_to_u8().unwrap();
    let rs_size = rs::Size::from_bits(n_bits);
    let sign_extended = rs_size.sign_extend(bits);
    if ity.signed == Signedness::Signed && sign_extended >> (n_bits - 1) != 0 {
        -Int::from(rs_size.truncate((!sign_extended).wrapping_add(1)))
    } else {
        Int::from(rs_size.truncate(sign_extended))
    }
}

pub fn discriminant_for_variant<'tcx>(ty: rs::Ty<'tcx>, tcx: rs::TyCtxt<'tcx>, variant_idx: rs::VariantIdx) -> Int {
    let rs::TyKind::Adt(adt_def, _) = ty.kind() else {
        panic!("Getting discriminant for a variant of a non-enum type!")
    };
    assert!(adt_def.is_enum());
    let Type::Int(discriminant_ty) = translate_ty(ty.discriminant_ty(tcx), tcx) else {
        panic!("Discriminant type is not integer!")
    };
    let discriminant = adt_def.discriminant_for_variant(tcx, variant_idx);
    int_from_bits(discriminant.val, discriminant_ty)
}
