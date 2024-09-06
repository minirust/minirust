use crate::*;

impl<'cx, 'tcx> FnCtxt<'cx, 'tcx> {
    pub fn translate_rvalue(&mut self, rv: &rs::Rvalue<'tcx>, span: rs::Span) -> ValueExpr {
        self.translate_rvalue_smir(&smir::stable(rv), span)
    }

    pub fn translate_rvalue_smir(&mut self, rv: &smir::Rvalue, span: rs::Span) -> ValueExpr {
        match rv {
            smir::Rvalue::Use(operand) => self.translate_operand_smir(operand, span),
            smir::Rvalue::BinaryOp(bin_op, l, r) => {
                let lty_smir = l.ty(&self.locals_smir).unwrap();
                let lty = self.translate_ty_smir(lty_smir, span);
                let rty_smir = r.ty(&self.locals_smir).unwrap();
                let rty = self.translate_ty_smir(rty_smir, span);

                let l = self.translate_operand_smir(l, span);
                let r = self.translate_operand_smir(r, span);

                use smir::BinOp::*;
                match (bin_op, lty) {
                    (Offset, Type::Ptr(_)) => {
                        // We have to convert the units.
                        let smir::TyKind::RigidTy(smir::RigidTy::RawPtr(pointee, _mutbl)) =
                            lty_smir.kind()
                        else {
                            unreachable!()
                        };
                        let pointee = self.rs_layout_of_smir(pointee);
                        assert!(pointee.is_sized());
                        let size = Int::from(pointee.size.bytes());
                        let size = ValueExpr::Constant(Constant::Int(size), rty);
                        let offset_bytes = build::mul_unchecked(r, size);
                        build::ptr_offset(l, offset_bytes, build::InBounds::Yes)
                    }

                    (Add, Type::Int(_)) => build::add(l, r),
                    (Sub, Type::Int(_)) => build::sub(l, r),
                    (Mul, Type::Int(_)) => build::mul(l, r),
                    (Div, Type::Int(_)) => build::div(l, r),
                    (Rem, Type::Int(_)) => build::rem(l, r),
                    (Shl, Type::Int(_)) => build::shl(l, r),
                    (Shr, Type::Int(_)) => build::shr(l, r),
                    (BitAnd, Type::Int(_)) => build::bit_and(l, r),
                    (BitOr, Type::Int(_)) => build::bit_or(l, r),
                    (BitXor, Type::Int(_)) => build::bit_xor(l, r),
                    (AddUnchecked, Type::Int(_)) => build::add_unchecked(l, r),
                    (SubUnchecked, Type::Int(_)) => build::sub_unchecked(l, r),
                    (MulUnchecked, Type::Int(_)) => build::mul_unchecked(l, r),
                    (ShlUnchecked, Type::Int(_)) => build::shl_unchecked(l, r),
                    (ShrUnchecked, Type::Int(_)) => build::shr_unchecked(l, r),

                    (Lt, _) => build::lt(l, r),
                    (Le, _) => build::le(l, r),
                    (Gt, _) => build::gt(l, r),
                    (Ge, _) => build::ge(l, r),
                    (Eq, _) => build::eq(l, r),
                    (Ne, _) => build::ne(l, r),

                    (Cmp, _) => {
                        let res = build::cmp(l, r);
                        // MiniRust expects an i8 for BinOp::Cmp but MIR uses an Ordering enum,
                        // so we have to transmute the result.
                        let ordering_ty: rs::Ty = self.tcx.ty_ordering_enum(None);
                        let ordering_ty: Type = self.translate_ty(ordering_ty, span);
                        build::transmute(res, ordering_ty)
                    }

                    (BitAnd, Type::Bool) => build::bool_and(l, r),
                    (BitOr, Type::Bool) => build::bool_or(l, r),
                    (BitXor, Type::Bool) => build::bool_xor(l, r),

                    (op, _) =>
                        rs::span_bug!(span, "Binary Op {op:?} not supported for type {lty_smir}."),
                }
            }
            smir::Rvalue::CheckedBinaryOp(op, l, r) => {
                let l = GcCow::new(self.translate_operand_smir(l, span));
                let r = GcCow::new(self.translate_operand_smir(r, span));

                let op = match op {
                    smir::BinOp::Add => BinOp::IntWithOverflow(IntBinOpWithOverflow::Add),
                    smir::BinOp::Sub => BinOp::IntWithOverflow(IntBinOpWithOverflow::Sub),
                    smir::BinOp::Mul => BinOp::IntWithOverflow(IntBinOpWithOverflow::Mul),
                    x => panic!("CheckedBinaryOp {x:?} not supported."),
                };
                ValueExpr::BinOp { operator: op, left: l, right: r }
            }
            smir::Rvalue::UnaryOp(unop, operand) => {
                let ty_smir = operand.ty(&self.locals_smir).unwrap();
                let ty = self.translate_ty_smir(ty_smir, span);
                let operand = self.translate_operand_smir(operand, span);

                use smir::UnOp::*;
                match (unop, ty) {
                    (Neg, Type::Int(_)) => build::neg(operand),
                    (Not, Type::Int(_)) => build::bit_not(operand),
                    (Not, Type::Bool) => build::not(operand),
                    (PtrMetadata, Type::Ptr(_)) => build::get_metadata(operand),
                    (op, _) =>
                        rs::span_bug!(span, "UnOp {op:?} called with unsupported type {ty_smir}."),
                }
            }
            smir::Rvalue::Ref(_, bkind, place) => {
                let ty = place.ty(&self.locals_smir).unwrap();
                let pointee = self.pointee_info_of_smir(ty, span);

                let place = self.translate_place_smir(place, span);
                let target = GcCow::new(place);
                let mutbl = translate_mutbl_smir(bkind.to_mutable_lossy());

                let ptr_ty = PtrType::Ref { mutbl, pointee };

                ValueExpr::AddrOf { target, ptr_ty }
            }
            smir::Rvalue::NullaryOp(null_op, ty) => {
                let ty = smir::internal(self.tcx, ty);
                let layout = self.rs_layout_of(ty);
                match null_op {
                    smir::NullOp::UbChecks => build::const_bool(self.tcx.sess.ub_checks()),
                    smir::NullOp::SizeOf => build::const_int(layout.size().bytes()),
                    smir::NullOp::AlignOf => build::const_int(layout.align().abi.bytes()),
                    smir::NullOp::OffsetOf(fields) => {
                        let param_env = rs::ParamEnv::reveal_all();
                        let ty_and_layout = self.tcx.layout_of(param_env.and(ty)).unwrap();
                        let fields = fields.iter().map(|field| {
                            (smir::internal(self.tcx, field.0), rs::FieldIdx::from_usize(field.1))
                        });
                        build::const_int(
                            self.tcx.offset_of_subfield(param_env, ty_and_layout, fields).bytes(),
                        )
                    }
                }
            }
            smir::Rvalue::AddressOf(_mutbl, place) => {
                let ty = place.ty(&self.locals_smir).unwrap();
                let place = self.translate_place_smir(place, span);
                let target = GcCow::new(place);
                let meta_kind = self.pointee_info_of_smir(ty, span).size.meta_kind();

                let ptr_ty = PtrType::Raw { meta_kind };

                ValueExpr::AddrOf { target, ptr_ty }
            }
            smir::Rvalue::Aggregate(agg, operands) => {
                let ty = rv.ty(&self.locals_smir).unwrap();
                let ty = self.translate_ty_smir(ty, span);
                match ty {
                    Type::Union { .. } => {
                        let smir::AggregateKind::Adt(_, _, _, _, Some(field_idx)) = agg else {
                            panic!()
                        };
                        assert_eq!(operands.len(), 1);
                        let expr = self.translate_operand_smir(&operands[0], span);
                        ValueExpr::Union {
                            field: (*field_idx).into(),
                            expr: GcCow::new(expr),
                            union_ty: ty,
                        }
                    }
                    Type::Tuple { .. } | Type::Array { .. } => {
                        let ops: List<_> =
                            operands.iter().map(|x| self.translate_operand_smir(x, span)).collect();
                        ValueExpr::Tuple(ops, ty)
                    }
                    Type::Enum { variants, .. } => {
                        let smir::AggregateKind::Adt(_, variant_idx, _, _, _) = agg else {
                            panic!()
                        };
                        let variant_ty = rv.ty(&self.locals_smir).unwrap();
                        let discriminant =
                            self.discriminant_for_variant_smir(variant_ty, *variant_idx, span);
                        let ops: List<_> =
                            operands.iter().map(|x| self.translate_operand_smir(x, span)).collect();

                        // We represent the multiple fields of an enum variant as a MiniRust tuple.
                        let data = GcCow::new(ValueExpr::Tuple(
                            ops,
                            variants.get(discriminant).unwrap().ty,
                        ));
                        ValueExpr::Variant { discriminant, data, enum_ty: ty }
                    }
                    Type::Ptr(PtrType::Raw { .. }) => {
                        if operands.len() != 2 {
                            rs::span_bug!(
                                span,
                                "Aggregating a pointer with {} operands",
                                operands.len()
                            );
                        }
                        let ptr = self.translate_operand_smir(&operands[0], span);
                        let meta = self.translate_operand_smir(&operands[1], span);

                        // We rely on MIR being well formed and matching our type-meta-pairings for this to be WF.
                        build::construct_wide_pointer(ptr, meta, ty)
                    }
                    x => rs::span_bug!(span, "Invalid aggregate type: {x:?}"),
                }
            }
            smir::Rvalue::CopyForDeref(place) =>
                ValueExpr::Load { source: GcCow::new(self.translate_place_smir(place, span)) },
            smir::Rvalue::Len(place) => {
                let ty = place.ty(&self.locals_smir).unwrap();
                match self.translate_ty_smir(ty, span) {
                    Type::Array { elem: _, count } => {
                        // FIXME: still evaluate the place -- it might have UB after all.
                        ValueExpr::Constant(Constant::Int(count), <usize>::get_type())
                    }
                    Type::Slice { .. } => {
                        // Convert the place to a value first, so our `get_metadata` is applicable.
                        build::get_metadata(build::addr_of(
                            self.translate_place_smir(place, span),
                            build::raw_ptr_ty(PointerMetaKind::ElementCount),
                        ))
                    }
                    _ => rs::span_bug!(span, "Rvalue::Len only supported for arrays & slices"),
                }
            }
            smir::Rvalue::Discriminant(place) =>
                ValueExpr::GetDiscriminant {
                    place: GcCow::new(self.translate_place_smir(place, span)),
                },
            smir::Rvalue::Repeat(op, c) => {
                let c = c.eval_target_usize().unwrap();
                let c = Int::from(c);

                let elem_ty = op.ty(&self.locals_smir).unwrap();
                let elem_ty = self.translate_ty_smir(elem_ty, span);
                let op = self.translate_operand_smir(op, span);

                let ty = Type::Array { elem: GcCow::new(elem_ty), count: c };

                let ls = list![op; c];
                ValueExpr::Tuple(ls, ty)
            }
            smir::Rvalue::Cast(cast_kind, operand, cast_ty) => {
                match cast_kind {
                    smir::CastKind::IntToInt => {
                        let operand_ty = operand.ty(&self.locals_smir).unwrap();
                        let operand_ty = self.translate_ty_smir(operand_ty, span);
                        let operand = self.translate_operand_smir(operand, span);
                        let Type::Int(int_ty) = self.translate_ty_smir(*cast_ty, span) else {
                            rs::span_bug!(span, "Attempting to IntToInt-Cast to non-int type!");
                        };

                        let operand = match operand_ty {
                            Type::Int(_) => operand,
                            // bool2int casts first go to u8, and then to the final type.
                            Type::Bool => build::transmute(operand, u8::get_type()),
                            _ =>
                                rs::span_bug!(
                                    span,
                                    "Attempting to cast non-int and non-boolean type to int!"
                                ),
                        };
                        ValueExpr::UnOp {
                            operator: UnOp::Cast(CastOp::IntToInt(int_ty)),
                            operand: GcCow::new(operand),
                        }
                    }

                    smir::CastKind::PtrToPtr => {
                        let operand_ty = operand.ty(&self.locals_smir).unwrap();
                        let Type::Ptr(PtrType::Raw { meta_kind: old_meta_kind }) =
                            self.translate_ty_smir(operand_ty, span)
                        else {
                            rs::span_bug!(span, "ptr to ptr cast on non-raw-pointer");
                        };
                        let Type::Ptr(PtrType::Raw { meta_kind: new_meta_kind }) =
                            self.translate_ty_smir(*cast_ty, span)
                        else {
                            rs::span_bug!(span, "ptr to ptr cast to non-raw-pointer");
                        };
                        let operand = self.translate_operand_smir(operand, span);
                        if old_meta_kind == new_meta_kind {
                            // Since raw pointers do only know care about the meta kind, no transmute is necessary here.
                            operand
                        } else if new_meta_kind == PointerMetaKind::None {
                            build::get_thin_pointer(operand)
                        } else {
                            rs::span_bug!(
                                span,
                                "PtrToPtr cast with wide target `{cast_ty:?}` that differes from source `{operand_ty:?}`"
                            );
                        }
                    }
                    smir::CastKind::PointerCoercion(smir::PointerCoercion::Unsize) => {
                        let operand_ty = operand.ty(&self.locals_smir).unwrap();
                        let old_pointee_rs_ty =
                            smir::internal(self.tcx, operand_ty).builtin_deref(true).unwrap();
                        let old_pointee_ty = self.translate_ty(old_pointee_rs_ty, span);
                        let new_pointee_rs_ty =
                            smir::internal(self.tcx, *cast_ty).builtin_deref(true).unwrap();
                        let new_pointee_ty = self.translate_ty(new_pointee_rs_ty, span);

                        let Type::Ptr(new_ptr_ty) = self.translate_ty_smir(*cast_ty, span) else {
                            rs::span_bug!(span, "ptr to ptr cast to non-pointer");
                        };
                        let operand = self.translate_operand_smir(operand, span);
                        match (old_pointee_ty, new_pointee_ty) {
                            (Type::Array { count, elem: a_elem }, Type::Slice { elem: s_elem }) => {
                                if a_elem != s_elem {
                                    rs::span_bug!(
                                        span,
                                        "Unsizing to slice with different element type"
                                    );
                                }
                                build::construct_wide_pointer(
                                    operand,
                                    build::const_int_typed::<usize>(count),
                                    Type::Ptr(new_ptr_ty),
                                )
                            }
                            _ =>
                                rs::span_bug!(
                                    span,
                                    "Unsupported unsizing coercion to {new_pointee_ty:?}"
                                ),
                        }
                    }
                    smir::CastKind::Transmute
                    | smir::CastKind::FnPtrToPtr
                    | smir::CastKind::PointerCoercion(smir::PointerCoercion::UnsafeFnPointer) => {
                        let operand = self.translate_operand_smir(operand, span);
                        let ty = self.translate_ty_smir(*cast_ty, span);
                        build::transmute(operand, ty)
                    }
                    smir::CastKind::PointerCoercion(smir::PointerCoercion::ReifyFnPointer) => {
                        let smir::Operand::Constant(f1) = operand else { panic!() };
                        let smir::TyKind::RigidTy(smir::RigidTy::FnDef(f, substs_ref)) =
                            f1.ty().kind()
                        else {
                            panic!()
                        };
                        let instance = smir::Instance::resolve(f, &substs_ref).unwrap();

                        build::fn_ptr_internal(self.cx.get_fn_name_smir(instance).0.get_internal())
                    }

                    smir::CastKind::PointerExposeAddress =>
                        unreachable!(
                            "PointerExposeAddress should have been handled on the statement level"
                        ),
                    smir::CastKind::PointerWithExposedProvenance =>
                        unreachable!(
                            "PointerWithExposedProvenance should have been handled on the statement level"
                        ),
                    smir::CastKind::PointerCoercion(
                        smir::PointerCoercion::MutToConstPointer
                        | smir::PointerCoercion::ArrayToPointer,
                    ) => unreachable!("{cast_kind:?} casts should not occur in runtime MIR"),

                    smir::CastKind::FloatToFloat
                    | smir::CastKind::FloatToInt
                    | smir::CastKind::IntToFloat
                    | smir::CastKind::DynStar
                    | smir::CastKind::PointerCoercion(smir::PointerCoercion::ClosureFnPointer(
                        ..,
                    )) => rs::span_bug!(span, "cast not supported: {cast_kind:?}"),
                }
            }

            smir::Rvalue::ShallowInitBox(..) | smir::Rvalue::ThreadLocalRef(..) =>
                rs::span_bug!(span, "rvalue not supported: {rv:?}"),
        }
    }

    pub fn translate_operand(&mut self, operand: &rs::Operand<'tcx>, span: rs::Span) -> ValueExpr {
        self.translate_operand_smir(&smir::stable(operand), span)
    }

    pub fn translate_operand_smir(&mut self, operand: &smir::Operand, span: rs::Span) -> ValueExpr {
        match operand {
            smir::Operand::Constant(c) => self.translate_const_smir(&c.const_, span),
            smir::Operand::Copy(place) =>
                ValueExpr::Load { source: GcCow::new(self.translate_place_smir(place, span)) },
            smir::Operand::Move(place) =>
                ValueExpr::Load { source: GcCow::new(self.translate_place_smir(place, span)) },
        }
    }

    pub fn translate_place(&mut self, place: &rs::Place<'tcx>, span: rs::Span) -> PlaceExpr {
        self.translate_place_smir(&smir::stable(place), span)
    }

    pub fn translate_place_smir(&mut self, place: &smir::Place, span: rs::Span) -> PlaceExpr {
        // Initial state: start with the local the place is based on
        let expr = PlaceExpr::Local(self.local_name_map[&place.local.into()]);
        let place_ty = self.locals_smir[place.local].ty;
        // Fold over all projections
        let (expr, _place_ty) =
            place.projection.iter().fold((expr, place_ty), |(expr, place_ty), proj| {
                let this_ty = proj.ty(place_ty).unwrap();
                let this_expr = match proj {
                    smir::ProjectionElem::Field(f, _ty) => {
                        let indirected = GcCow::new(expr);
                        PlaceExpr::Field { root: indirected, field: (*f).into() }
                    }
                    smir::ProjectionElem::Deref => {
                        let x = GcCow::new(expr);
                        let x = ValueExpr::Load { source: x };
                        let x = GcCow::new(x);

                        let ty = self.translate_ty_smir(this_ty, span);

                        PlaceExpr::Deref { operand: x, ty }
                    }
                    smir::ProjectionElem::Index(loc) => {
                        let i = PlaceExpr::Local(self.local_name_map[&(*loc).into()]);
                        let i = GcCow::new(i);
                        let i = ValueExpr::Load { source: i };
                        let i = GcCow::new(i);
                        let root = GcCow::new(expr);
                        PlaceExpr::Index { root, index: i }
                    }
                    smir::ProjectionElem::Downcast(variant_idx) => {
                        let root = GcCow::new(expr);
                        let discriminant =
                            self.discriminant_for_variant_smir(this_ty, *variant_idx, span);
                        PlaceExpr::Downcast { root, discriminant }
                    }

                    stable_mir::mir::ProjectionElem::ConstantIndex { .. }
                    | stable_mir::mir::ProjectionElem::Subslice { .. }
                    | stable_mir::mir::ProjectionElem::OpaqueCast(_)
                    | stable_mir::mir::ProjectionElem::Subtype(_) => {
                        rs::span_bug!(span, "Place Projection not supported: {:?}", proj);
                    }
                };
                (this_expr, this_ty)
            });
        expr
    }
}
