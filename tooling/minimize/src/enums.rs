use crate::*;
mod rs {
    pub use crate::rs::*;
    pub use crate::rustc_target::abi::{Variants, FieldsShape, Primitive, TagEncoding, VariantIdx};
}

use crate::rustc_middle::ty::layout::PrimitiveExt;

pub fn translate_enum<'tcx>(
    ty: rs::Ty<'tcx>,
    adt_def: rs::AdtDef<'tcx>,
    sref: rs::GenericArgsRef<'tcx>,
    tcx: rs::TyCtxt<'tcx>,
) -> Type {
    let a = rs::ParamEnv::reveal_all().and(ty);
    let layout = tcx.layout_of(a).unwrap().layout;
    let size = translate_size(layout.size());
    let align = translate_align(layout.align().abi);

    let Type::Int(discriminant_ty) = translate_ty(ty.discriminant_ty(tcx), tcx) else {
        panic!("Discriminant type is not integer!")
    };

    let (variants, discriminator) = match layout.variants() {
        rs::Variants::Single { index } => {
            let fields = translate_fields(layout.fields(), adt_def.variant(*index), sref, tcx);
            let variants = [(Int::ZERO, Variant { ty: Type::Tuple { fields, size, align }, tagger: Map::new() })];
            let discriminator = Discriminator::Known(Int::ZERO);
            (variants.into_iter().collect::<Map<Int, Variant>>(), discriminator)
        },
        rs::Variants::Multiple {
            tag,
            tag_encoding,
            tag_field,
            variants,
        } => {

            // compute the offset of the tag for the tagger and discriminator construction
            let tag_offset: Offset = translate_size(layout.fields().offset(*tag_field));
            let Type::Int(tag_ty) = translate_ty(tag.primitive().to_int_ty(tcx), tcx) else {
                panic!("enum tag has invalid primitive type")
            };

            // translate the variants
            let mut translated_variants = Map::new();
            let mut discriminator_children = Map::new();
            for (variant_idx, variant_def) in adt_def.variants().iter_enumerated() {
                let fields = translate_fields(&variants[variant_idx].fields, &variant_def, sref, tcx);
                let discr = adt_def.discriminant_for_variant(tcx, variant_idx);
                let discr_int = int_from_bits(discr.val, discriminant_ty);
                match tag_encoding {
                    rs::TagEncoding::Direct => {
                        // direct tagging places the discriminant in the tag for all variants
                        let tagger = [(tag_offset, (tag_ty, discr_int))].into_iter().collect::<Map<Offset, (IntType, Int)>>();
                        let variant = Variant { ty: Type::Tuple { fields, size, align }, tagger };
                        translated_variants.insert(discr_int, variant);
                        discriminator_children.insert(discr_int, Discriminator::Known(discr_int));
                    },
                    rs::TagEncoding::Niche { untagged_variant, niche_variants, niche_start } if *untagged_variant != variant_idx => {
                        // this is a tagged variant
                        let rsize = rs::Size::from_bytes(tag_ty.size.bytes().try_to_u8().unwrap());
                        let discr_bits = if tag_ty.signed == Signedness::Signed { rsize.sign_extend(discr.val) } else { discr.val };
                        let tag = (discr_bits - niche_variants.start().as_usize() as u128).wrapping_add(*niche_start);
                        let tag_int = int_from_bits(tag, tag_ty);
                        let tagger = [(tag_offset, (tag_ty, tag_int))].into_iter().collect::<Map<_, _>>();
                        discriminator_children.insert((tag_int, tag_int), Discriminator::Known(discr_int));
                        translated_variants.insert(discr_int, Variant { ty: Type::Tuple { fields, size, align }, tagger });
                    }
                    rs::TagEncoding::Niche { .. } => {
                        // this is the untagged variant
                        // First we need to ensure that we can use the valid niche values to detect this variant.
                        // Since the niche type is at the moment directly used for the tag by the compiler this should be no issue.
                        let niche = variants[variant_idx].largest_niche.expect("Untagged variant has no Niche in a multiple variant enum!");
                        assert!(niche.offset.bytes() == tag_offset.bytes().try_to_usize().unwrap() as u64, "Untagged variant has niche at different offset than tag.");
                        let niche_ty = translate_tag_primitive(niche.value, &tcx);
                        assert!(niche_ty == tag_ty, "Niche of untagged variant is of different type");

                        // Insert the discriminator children if the range wraps or the single child otherwise.
                        let start = int_from_bits(niche.valid_range.start, niche_ty);
                        let end = int_from_bits(niche.valid_range.end, niche_ty);
                        if niche.valid_range.start > niche.valid_range.end {
                            let bits = niche_ty.size.bits().try_to_u8().unwrap();
                            let (min, max) = if niche_ty.signed == Signedness::Signed {
                                // signed bit patterns: min = 0b100...0, max = 0b011...1
                                (int_from_bits(1u128 << (bits - 1), niche_ty), int_from_bits(!0u128 >> (129 - bits), niche_ty))
                            } else {
                                // unsigned bit patterns: min = 0b000...0 = 0, max = 0b111...1
                                (Int::ZERO, int_from_bits(u128::MAX, niche_ty))
                            };
                            discriminator_children.insert((start, max), Discriminator::Known(discr_int));
                            discriminator_children.insert((min, end), Discriminator::Known(discr_int));
                        } else {
                            discriminator_children.insert((start, end), Discriminator::Known(discr_int));
                        };
                        translated_variants.insert(discr_int, Variant { ty: Type::Tuple { fields, size, align }, tagger: Map::new() });
                    }
                };
            }

            let discriminator = Discriminator::Branch {
                offset: tag_offset,
                value_type: tag_ty,
                fallback: GcCow::new(Discriminator::Invalid),
                children: discriminator_children
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
    let rs_size = rs::Size::from_bits(ity.size.bits().try_to_u8().unwrap());
    if ity.signed == Signedness::Unsigned {
        Int::from(rs_size.truncate(bits))
    } else {
        let signed_val = rs_size.sign_extend(bits) as i128;
        Int::from(signed_val)
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
