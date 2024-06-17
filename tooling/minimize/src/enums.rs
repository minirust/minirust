use crate::*;

use crate::rustc_middle::ty::layout::PrimitiveExt;

impl<'tcx> Ctxt<'tcx> {
    pub fn translate_enum(
        &self,
        ty: rs::Ty<'tcx>,
        adt_def: rs::AdtDef<'tcx>,
        sref: rs::GenericArgsRef<'tcx>,
        span: rs::Span,
    ) -> Type {
        let layout = self.rs_layout_of(ty);
        let size = translate_size(layout.size());
        let align = translate_align(layout.align().abi);

        let Type::Int(discriminant_ty) = self.translate_ty(ty.discriminant_ty(self.tcx), span)
        else {
            panic!("Discriminant type is not integer!")
        };

        let (variants, discriminator) = match layout.variants() {
            rs::Variants::Single { index } => {
                let fields = self.translate_adt_variant_fields(
                    layout.fields(),
                    adt_def.variant(*index),
                    sref,
                    span,
                );
                let variants = [(
                    Int::ZERO,
                    Variant { ty: Type::Tuple { fields, size, align }, tagger: Map::new() },
                )];
                let discriminator = Discriminator::Known(Int::ZERO);
                (variants.into_iter().collect::<Map<Int, Variant>>(), discriminator)
            }
            rs::Variants::Multiple { tag, tag_encoding, tag_field, variants } => {
                // compute the offset of the tag for the tagger and discriminator construction
                let tag_offset: Offset = translate_size(layout.fields().offset(*tag_field));
                let Type::Int(tag_ty) =
                    self.translate_ty(tag.primitive().to_int_ty(self.tcx), span)
                else {
                    panic!("enum tag has invalid primitive type")
                };

                // translate the variants
                let mut translated_variants = Map::new();
                let mut discriminator_branches = Map::new();
                for (variant_idx, variant_def) in adt_def.variants().iter_enumerated() {
                    let fields = self.translate_adt_variant_fields(
                        &variants[variant_idx].fields,
                        &variant_def,
                        sref,
                        span,
                    );
                    let discr = adt_def.discriminant_for_variant(self.tcx, variant_idx);
                    let discr_int = int_from_bits(discr.val, discriminant_ty);
                    match tag_encoding {
                        rs::TagEncoding::Direct => {
                            // direct tagging places the discriminant in the tag for all variants
                            let tagger = [(tag_offset, (tag_ty, discr_int))]
                                .into_iter()
                                .collect::<Map<Offset, (IntType, Int)>>();
                            let variant =
                                Variant { ty: Type::Tuple { fields, size, align }, tagger };
                            translated_variants.insert(discr_int, variant);
                            discriminator_branches.insert(
                                (discr_int, discr_int + Int::ONE),
                                Discriminator::Known(discr_int),
                            );
                        }
                        rs::TagEncoding::Niche {
                            untagged_variant,
                            niche_variants,
                            niche_start,
                        } if *untagged_variant != variant_idx => {
                            // this is a tagged variant, meaning that it writes its tag and has a discriminator branch entry.
                            let discr_int = int_from_bits(discr.val, tag_ty);
                            let tag_int = (discr_int
                                - Int::from(niche_variants.start().as_usize())
                                + Int::from(*niche_start))
                            .bring_in_bounds(tag_ty.signed, tag_ty.size);
                            let tagger = [(tag_offset, (tag_ty, tag_int))]
                                .into_iter()
                                .collect::<Map<_, _>>();
                            discriminator_branches.insert(
                                (tag_int, tag_int + Int::ONE),
                                Discriminator::Known(discr_int),
                            );
                            translated_variants.insert(
                                discr_int,
                                Variant { ty: Type::Tuple { fields, size, align }, tagger },
                            );
                        }
                        rs::TagEncoding::Niche { .. } => {
                            // this is the untagged variant
                            // we don't add it to the discriminator branches as it will be the fallback.
                            translated_variants.insert(
                                discr_int,
                                Variant {
                                    ty: Type::Tuple { fields, size, align },
                                    tagger: Map::new(),
                                },
                            );
                        }
                    };
                }

                let fallback = match tag_encoding {
                    // Direct tagging: all other tag values are invalid.
                    rs::TagEncoding::Direct => GcCow::new(Discriminator::Invalid),

                    // Niche tagging: The fallback is the untagged variant.
                    // But we still want to declare unexpected values and invalid.
                    // So we check the valid range of the tag, and add discriminator branches for everything *outside*
                    // that range to declare it invalid.
                    rs::TagEncoding::Niche { untagged_variant, .. } => {
                        let tag_valid_range = tag.valid_range(&self.tcx);
                        let start = int_from_bits(tag_valid_range.start, tag_ty);
                        let end = int_from_bits(tag_valid_range.end, tag_ty);
                        if start <= end {
                            // The range of valid values is continuous, so the invalid values are between the ends of the range and the domain.
                            let rsize = tag.size(&self.tcx);
                            let min = if tag_ty.signed == Signedness::Signed {
                                Int::from(rsize.signed_int_min())
                            } else {
                                Int::ZERO
                            };
                            let max = if tag_ty.signed == Signedness::Signed {
                                Int::from(rsize.signed_int_max())
                            } else {
                                Int::from(rsize.unsigned_int_max())
                            };
                            if end < max {
                                discriminator_branches.insert(
                                    (end + Int::ONE, max + Int::ONE),
                                    Discriminator::Invalid,
                                );
                            }
                            if min < start {
                                discriminator_branches.insert((min, start), Discriminator::Invalid);
                            }
                        } else if end + Int::ONE < start {
                            // The range of valid values wraps around, so the invalid values are between end and start (exclusive).
                            discriminator_branches
                                .insert((end + Int::ONE, start), Discriminator::Invalid);
                        } else {
                        }

                        GcCow::new(Discriminator::Known(untagged_variant.as_usize().into()))
                    }
                };
                let discriminator = Discriminator::Branch {
                    offset: tag_offset,
                    value_type: tag_ty,
                    fallback,
                    children: discriminator_branches,
                };

                (translated_variants, discriminator)
            }
        };

        Type::Enum { variants, discriminator, discriminant_ty, size, align }
    }

    pub fn discriminant_for_variant_smir(
        &self,
        ty: smir::Ty,
        variant_idx: smir::VariantIdx,
        span: rs::Span,
    ) -> Int {
        self.discriminant_for_variant(
            smir::internal(self.tcx, ty),
            smir::internal(self.tcx, variant_idx),
            span,
        )
    }

    pub fn discriminant_for_variant(
        &self,
        ty: rs::Ty<'tcx>,
        variant_idx: rs::VariantIdx,
        span: rs::Span,
    ) -> Int {
        let rs::TyKind::Adt(adt_def, _) = ty.kind() else {
            panic!("Getting discriminant for a variant of a non-enum type!")
        };
        assert!(adt_def.is_enum());
        let Type::Int(discriminant_ty) = self.translate_ty(ty.discriminant_ty(self.tcx), span)
        else {
            panic!("Discriminant type is not integer!")
        };
        let discriminant = adt_def.discriminant_for_variant(self.tcx, variant_idx);
        int_from_bits(discriminant.val, discriminant_ty)
    }
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
