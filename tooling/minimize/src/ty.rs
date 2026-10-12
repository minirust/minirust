use rustc_middle::ty::Unnormalized;

use crate::*;

impl<'tcx> Ctxt<'tcx> {
    pub fn pointee_info_of(&mut self, ty: rs::Ty<'tcx>, span: rs::Span) -> PointeeInfo {
        let layout = self.rs_layout_of(ty).layout;
        let inhabited = !layout.is_uninhabited();
        let freeze = ty.is_freeze(self.tcx, self.typing_env());
        let unpin = ty.is_unpin(self.tcx, self.typing_env());

        if layout.is_sized() {
            let size = translate_size(layout.size());
            let align = translate_align(layout.align().abi);
            let layout = LayoutStrategy::Sized(size, align);

            // Because we compute `cell_bytes` by iterating through the fields of
            // the type in declaration, not in memory order, the order of the ranges are
            // not necessarily sorted in ascending order.
            let mut cells = self.cells_in_sized_ty(ty, span);
            cells.sort_by_key(|a| a.0);
            let cells = cells.into_iter().collect::<List<(Offset, Offset)>>();

            return PointeeInfo {
                layout,
                inhabited,
                unsafe_cells: UnsafeCellStrategy::Sized { cells },
                freeze,
                unpin,
            };
        }

        // Handle Unsized types:
        match ty.kind() {
            &rs::TyKind::Slice(element_ty) => {
                let element_layout = self.rs_layout_of(element_ty).layout;
                let mut element_cells = self.cells_in_sized_ty(element_ty, span);
                element_cells.sort_by_key(|a| a.0);
                let element_cells = element_cells.into_iter().collect::<List<(Offset, Offset)>>();

                let size = translate_size(element_layout.size());
                let align = translate_align(element_layout.align().abi);
                let layout = LayoutStrategy::Slice(size, align);
                PointeeInfo {
                    layout,
                    inhabited,
                    unsafe_cells: UnsafeCellStrategy::Slice { element_cells },
                    freeze,
                    unpin,
                }
            }
            &rs::TyKind::Str => {
                // Treat `str` like `[u8]`.
                let layout = LayoutStrategy::Slice(Size::from_bytes_const(1), Align::ONE);
                PointeeInfo {
                    layout,
                    inhabited,
                    unsafe_cells: UnsafeCellStrategy::Slice { element_cells: List::new() },
                    freeze,
                    unpin,
                }
            }
            &rs::TyKind::Dynamic(_, _) => {
                let layout = LayoutStrategy::TraitObject(self.get_trait_name(ty));
                PointeeInfo {
                    layout,
                    inhabited,
                    unsafe_cells: UnsafeCellStrategy::TraitObject,
                    freeze,
                    unpin,
                }
            }
            _ => rs::span_bug!(span, "encountered unimplemented unsized type: {ty}"),
        }
    }

    pub fn pointee_info_of_smir(&mut self, ty: smir::Ty, span: rs::Span) -> PointeeInfo {
        self.pointee_info_of(smir::internal(self.tcx, ty), span)
    }

    pub fn translate_ty_smir(&mut self, ty: smir::Ty, span: rs::Span) -> Type {
        self.translate_ty(smir::internal(self.tcx, ty), span)
    }

    fn cells_from_layout(
        &mut self,
        layout: rs::TyAndLayout<'tcx>,
        span: rs::Span,
    ) -> Vec<(Offset, Size)> {
        (0..layout.fields.count())
            .flat_map(|i| {
                let offset = translate_size(layout.fields.offset(i));
                let ty = layout.field(self, i).ty;
                self.cells_in_sized_ty(ty, span)
                    .into_iter()
                    .map(move |(start, len)| (start + offset, len))
            })
            .collect()
    }

    pub fn cells_in_sized_ty(&mut self, ty: rs::Ty<'tcx>, span: rs::Span) -> Vec<(Offset, Size)> {
        match ty.kind() {
            rs::TyKind::Bool => Vec::new(),
            rs::TyKind::Int(_) => Vec::new(),
            rs::TyKind::Uint(_) => Vec::new(),
            rs::TyKind::Char => Vec::new(),
            rs::TyKind::RawPtr(..) => Vec::new(),
            rs::TyKind::Ref(..) => Vec::new(),
            rs::TyKind::Adt(adt_def, _) if adt_def.is_box() => Vec::new(),
            rs::TyKind::FnPtr(..) => Vec::new(),
            rs::TyKind::FnDef(..) => Vec::new(),
            rs::TyKind::Never => Vec::new(),
            rs::TyKind::Tuple(..) => {
                let layout = self.rs_layout_of(ty);
                self.cells_from_layout(layout, span)
            }
            rs::TyKind::Adt(adt_def, _) if adt_def.is_unsafe_cell() => {
                let layout = self.rs_layout_of(ty);
                let size = translate_size(layout.size);
                vec![(Size::ZERO, size)]
            }
            rs::TyKind::Adt(adt_def, _sref) if adt_def.is_struct() => {
                let layout = self.rs_layout_of(ty);
                self.cells_from_layout(layout, span)
            }
            rs::TyKind::Adt(adt_def, _sref) if adt_def.is_union() || adt_def.is_enum() => {
                // If any variant has an `UnsafeCell` somewhere in it, the whole range will be non-freeze.
                let ty_is_freeze = ty.is_freeze(self.tcx, self.typing_env());
                let layout = self.rs_layout_of(ty);
                let size = translate_size(layout.size);

                if ty_is_freeze { Vec::new() } else { vec![(Size::ZERO, size)] }
            }
            rs::TyKind::Closure(..) => {
                let layout = self.rs_layout_of(ty);
                self.cells_from_layout(layout, span)
            }
            rs::TyKind::Array(elem_ty, c) => {
                let range = self.cells_in_sized_ty(*elem_ty, span);
                if !range.is_empty() {
                    let layout = self.rs_layout_of(*elem_ty);
                    let size = translate_size(layout.size);
                    let count = c.try_to_target_usize(self.tcx).unwrap();
                    let ranges = vec![0, count];

                    ranges
                        .iter()
                        .enumerate()
                        .flat_map(|(i, _)| {
                            let offset = size * i.into();
                            range.iter().map(move |&(start, len)| (start + offset, len))
                        })
                        .collect()
                } else {
                    Vec::new()
                }
            }
            // pattern types do not affect the UnsafeCell positions
            rs::TyKind::Pat(ty, _) => self.cells_in_sized_ty(*ty, span),
            x => rs::span_bug!(span, "cells_in_sized_ty: TyKind not supported: {x:?}"),
        }
    }

    fn tuple_from_layout(&mut self, layout: rs::TyAndLayout<'tcx>, span: rs::Span) -> Type {
        let size = translate_size(layout.size);
        let align = translate_align(layout.align.abi);

        let fields: Vec<(Size, Type)> = (0..layout.fields.count())
            .map(|i| {
                let ty = layout.field(self, i).ty;
                let ty = self.translate_ty(ty, span);
                let offset = layout.fields.offset(i);
                let offset = translate_size(offset);
                (offset, ty)
            })
            .collect();

        build::tuple_ty(&fields, size, align)
    }

    pub fn translate_ty(&mut self, ty: rs::Ty<'tcx>, span: rs::Span) -> Type {
        if let Some(mini_ty) = self.ty_cache.get(&ty) {
            return *mini_ty;
        }

        let mini_ty = match ty.kind() {
            rs::TyKind::Bool => Type::Bool,
            rs::TyKind::Int(t) => {
                let sz = rs::abi::Integer::from_int_ty(&self.tcx, *t).size();
                build::int_ty(Signedness::Signed, translate_size(sz))
            }
            rs::TyKind::Uint(t) => {
                let sz = rs::abi::Integer::from_uint_ty(&self.tcx, *t).size();
                build::int_ty(Signedness::Unsigned, translate_size(sz))
            }
            rs::TyKind::Char => {
                let ty = IntType { size: Size::from_bytes_const(4), signed: Signedness::Unsigned };
                let niches = list![
                    (Int::from(0xD800), Int::from(0xE000)),
                    (Int::from(0x110000), Int::from(u32::MAX) + Int::ONE)
                ];
                Type::Int { ty, niches }
            }
            rs::TyKind::Pat(inner, pat) => self.translate_pat(*inner, *pat, span),
            rs::TyKind::Tuple(..) => {
                let layout = self.rs_layout_of(ty);
                self.tuple_from_layout(layout, span)
            }
            rs::TyKind::Adt(adt_def, _) if adt_def.is_box() => {
                let ty = ty.expect_boxed_ty();
                let pointee = self.pointee_info_of(ty, span);
                Type::Ptr(PtrType::Box { pointee })
            }
            rs::TyKind::Adt(adt_def, sref) if adt_def.is_struct() => {
                let (fields, size, align) = self.translate_non_enum_adt(ty, *adt_def, sref, span);
                build::tuple_ty(&fields.iter().collect::<Vec<_>>(), size, align)
            }
            rs::TyKind::Adt(adt_def, sref) if adt_def.is_union() => {
                let (fields, size, align) = self.translate_non_enum_adt(ty, *adt_def, sref, span);
                let chunks = calc_chunks(fields, size);
                Type::Union { fields, size, align, chunks }
            }
            rs::TyKind::Adt(adt_def, sref) if adt_def.is_enum() =>
                self.translate_enum(ty, *adt_def, sref, span),
            rs::TyKind::Ref(_, ty, mutbl) => {
                let pointee = self.pointee_info_of(*ty, span);
                let mutbl = translate_mutbl(*mutbl);
                Type::Ptr(PtrType::Ref { pointee, mutbl })
            }
            rs::TyKind::RawPtr(ty, _mutbl) => {
                let pointee = self.pointee_info_of(*ty, span);
                Type::Ptr(PtrType::Raw { meta_kind: pointee.layout.meta_kind(), may_be_null: true })
            }
            rs::TyKind::Array(ty, c) => {
                let count = Int::from(c.try_to_target_usize(self.tcx).unwrap());
                let elem = GcCow::new(self.translate_ty(*ty, span));
                Type::Array { elem, count }
            }
            rs::TyKind::FnPtr(..) => Type::Ptr(PtrType::FnPtr),
            rs::TyKind::FnDef(..) => {
                // FnDef types don't carry data, everything relevant is in the type
                // (which we handle when translating calls).
                self.translate_ty(self.tcx.types.unit, span)
            }
            rs::TyKind::Closure(..) => {
                let layout = self.rs_layout_of(ty);
                self.tuple_from_layout(layout, span)
            }
            rs::TyKind::Never =>
                build::enum_ty::<u8>(&[], Discriminator::Invalid, build::size(0), build::align(1)),
            rs::TyKind::Slice(ty) => {
                let elem = GcCow::new(self.translate_ty(*ty, span));
                Type::Slice { elem }
            }
            rs::TyKind::Str => {
                // Treat `str` like `[u8]`.
                let elem = GcCow::new(build::int_ty(
                    Signedness::Unsigned,
                    Size::from_bytes_const(1),
                ));
                Type::Slice { elem }
            }
            rs::TyKind::Dynamic(_, _) => Type::TraitObject(self.get_trait_name(ty)),
            x => rs::span_bug!(span, "TyKind not supported: {x:?}"),
        };
        self.ty_cache.insert(ty, mini_ty);
        mini_ty
    }

    /// Constructs the fields of a given variant.
    pub fn translate_adt_variant_fields(
        &mut self,
        shape: Option<&rs::FieldsShape<rs::FieldIdx>>,
        variant: &rs::VariantDef,
        sref: rs::GenericArgsRef<'tcx>,
        span: rs::Span,
    ) -> Fields {
        variant
            .fields
            .iter_enumerated()
            .map(|(i, field)| {
                let ty = field.ty(self.tcx, sref);
                // Field types can be non-normalized even if the ADT type was normalized
                // (due to associated types on the fields).
                let ty = self
                    .tcx
                    .normalize_erasing_regions(self.typing_env(), Unnormalized::new_wip(ty));
                let ty = self.translate_ty(ty, span);
                let offset = match shape {
                    Some(shape) => shape.offset(i.into()),
                    None => {
                        // Fields in an elided enum variant. Must all be zero-sized at offset zero.
                        assert!(
                            ty.layout::<DefaultTarget>().expect_size("broken enum") == Size::ZERO
                        );
                        rs::Size::ZERO
                    }
                };
                let offset = translate_size(offset);

                (offset, ty)
            })
            .collect()
    }

    fn translate_non_enum_adt(
        &mut self,
        ty: rs::Ty<'tcx>,
        adt_def: rs::AdtDef<'tcx>,
        sref: rs::GenericArgsRef<'tcx>,
        span: rs::Span,
    ) -> (Fields, Size, Align) {
        let layout = self.rs_layout_of(ty).layout;
        let fields = self.translate_adt_variant_fields(
            Some(layout.fields()),
            adt_def.non_enum_variant(),
            sref,
            span,
        );
        let size = translate_size(layout.size());
        let align = translate_align(layout.align().abi);

        (fields, size, align)
    }

    fn translate_pat(
        &mut self,
        inner: rs::Ty<'tcx>,
        pat: rs::Pattern<'tcx>,
        span: rs::Span,
    ) -> Type {
        let inner_ty = self.translate_ty(inner, span);
        match *pat {
            rs::PatternKind::NotNull => {
                let Type::Ptr(PtrType::Raw { meta_kind, may_be_null: _ }) = inner_ty else {
                    rs::span_bug!(span, "`!null` pattern applied to non-raw-pointer type: {inner}");
                };
                Type::Ptr(PtrType::Raw { meta_kind, may_be_null: false })
            }
            rs::PatternKind::Range { .. } | rs::PatternKind::Or(..) => {
                let Type::Int { ty: int_ty, niches: inner_niches } = inner_ty else {
                    rs::span_bug!(span, "range pattern applied to non-integer type: {inner}");
                };
                let eval_range = |p: rs::PatternKind<'tcx>| -> Option<(Int, Int)> {
                    let rs::PatternKind::Range { start, end } = p else {
                        rs::span_bug!(span, "unsupported pattern in integer pattern type: {p:?}");
                    };
                    let eval_const = |c: rustc_middle::ty::Const<'tcx>| -> Int {
                        let c = self
                            .tcx
                            .normalize_erasing_regions(self.typing_env(), Unnormalized::new_wip(c));
                        let bits = c.try_to_value().unwrap().to_leaf().to_bits_unchecked();
                        int_from_bits(bits, int_ty)
                    };
                    let s = eval_const(start);
                    let e = eval_const(end);
                    (s <= e).then_some((s, e + Int::ONE))
                };
                let mut valid_ranges: Vec<(Int, Int)> = match *pat {
                    rs::PatternKind::Range { .. } => eval_range(*pat).into_iter().collect(),
                    rs::PatternKind::Or(pats) => pats.iter().filter_map(|p| eval_range(*p)).collect(),
                    rs::PatternKind::NotNull => unreachable!(),
                };
                let valid_ranges = merge_intervals(&mut valid_ranges);

                let (min_val, max_val) = crate::enums::int_bounds(int_ty);
                let max_val_excl = max_val + Int::ONE;

                let mut all_niches: Vec<(Int, Int)> = inner_niches.iter().collect();
                let mut cursor = min_val;
                for (v_start, v_end) in valid_ranges {
                    if cursor < v_start {
                        all_niches.push((cursor, v_start));
                    }
                    if cursor < v_end {
                        cursor = v_end;
                    }
                }
                if cursor < max_val_excl {
                    all_niches.push((cursor, max_val_excl));
                }
                let niches = merge_intervals(&mut all_niches).into_iter().collect();
                Type::Int { ty: int_ty, niches }
            }
        }
    }
}

pub fn translate_mutbl(mutbl: rs::Mutability) -> Mutability {
    match mutbl {
        rs::Mutability::Mut => Mutability::Mutable,
        rs::Mutability::Not => Mutability::Immutable,
    }
}

pub fn translate_mutbl_smir(mutbl: smir::Mutability) -> Mutability {
    match mutbl {
        smir::Mutability::Mut => Mutability::Mutable,
        smir::Mutability::Not => Mutability::Immutable,
    }
}

pub fn translate_size(size: rs::Size) -> Size {
    Size::from_bytes_const(size.bytes())
}

pub fn translate_align(align: rs::Align) -> Align {
    Align::from_bytes(align.bytes()).unwrap()
}

pub fn translate_calling_convention(conv: rs::CanonAbi) -> CallingConvention {
    match conv {
        rs::CanonAbi::C => CallingConvention::C,
        rs::CanonAbi::Rust => CallingConvention::Rust,
        _ => todo!(),
    }
}

fn merge_intervals(intervals: &mut Vec<(Int, Int)>) -> Vec<(Int, Int)> {
    intervals.retain(|(s, e)| s < e);
    intervals.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let mut merged: Vec<(Int, Int)> = Vec::with_capacity(intervals.len());
    for &(s, e) in intervals.iter() {
        match merged.last_mut() {
            Some(last) if s <= last.1 => {
                if last.1 < e {
                    last.1 = e;
                }
            }
            _ => merged.push((s, e)),
        }
    }
    merged
}
