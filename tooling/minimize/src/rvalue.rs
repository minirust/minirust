use crate::*;

impl<'cx, 'tcx> FnCtxt<'cx, 'tcx> {
    /// Translate an rvalue -- could generate a bunch of helper statements.
    pub fn translate_rvalue(
        &mut self,
        rv: &rs::Rvalue<'tcx>,
    ) -> Option<(Vec<Statement>, ValueExpr)> {
        self.translate_rvalue_smir(&smir::stable(rv))
    }

    pub fn translate_rvalue_smir(
        &mut self,
        rv: &smir::Rvalue,
    ) -> Option<(Vec<Statement>, ValueExpr)> {
        Some((
            vec![],
            match rv {
                smir::Rvalue::Use(operand) => self.translate_operand_smir(operand),
                smir::Rvalue::CheckedBinaryOp(bin_op, l, r)
                | smir::Rvalue::BinaryOp(bin_op, l, r) => {
                    let lty = l.ty(&self.locals_smir).unwrap();
                    let rty = r.ty(&self.locals_smir).unwrap();

                    assert_eq!(lty, rty);

                    let l = self.translate_operand_smir(l);
                    let r = self.translate_operand_smir(r);

                    let l = GcCow::new(l);
                    let r = GcCow::new(r);

                    use smir::BinOp::*;
                    let op = if *bin_op == Offset {
                        BinOp::PtrOffset { inbounds: true }
                    } else {
                        // everything else right-now is a int op!

                        let op = |x| {
                            let Type::Int(int_ty) = self.translate_ty_smir(lty) else {
                                panic!("arithmetic operation with non-int type unsupported!");
                            };

                            BinOp::Int(x, int_ty)
                        };
                        let rel = |x| BinOp::IntRel(x);

                        match bin_op {
                            Add => op(BinOpInt::Add),
                            Sub => op(BinOpInt::Sub),
                            Mul => op(BinOpInt::Mul),
                            Div => op(BinOpInt::Div),
                            Rem => op(BinOpInt::Rem),

                            Lt => rel(IntRel::Lt),
                            Le => rel(IntRel::Le),
                            Gt => rel(IntRel::Gt),
                            Ge => rel(IntRel::Ge),
                            Eq => rel(IntRel::Eq),
                            Ne => rel(IntRel::Ne),

                            BitAnd => return None,
                            x => {
                                dbg!(x);
                                todo!("unsupported BinOp")
                            }
                        }
                    };

                    ValueExpr::BinOp { operator: op, left: l, right: r }
                }
                smir::Rvalue::UnaryOp(unop, operand) =>
                    match unop {
                        smir::UnOp::Neg => {
                            let ty = operand.ty(&self.locals_smir).unwrap();
                            let ty = self.translate_ty_smir(ty);
                            let Type::Int(int_ty) = ty else {
                                panic!("Neg operation with non-int type!");
                            };

                            let operand = self.translate_operand_smir(operand);

                            ValueExpr::UnOp {
                                operator: UnOp::Int(UnOpInt::Neg, int_ty),
                                operand: GcCow::new(operand),
                            }
                        }
                        smir::UnOp::Not => {
                            let ty = operand.ty(&self.locals_smir).unwrap();
                            let ty = self.translate_ty_smir(ty);
                            let Type::Bool = ty else {
                                panic!("Not operation with non-boolean type!");
                            };

                            let operand = self.translate_operand_smir(operand);

                            ValueExpr::UnOp {
                                operator: UnOp::Bool(UnOpBool::Not),
                                operand: GcCow::new(operand),
                            }
                        }
                    },
                smir::Rvalue::Ref(_, bkind, place) => {
                    let ty = place.ty(&self.locals_smir).unwrap();
                    let pointee = self.layout_of_smir(ty);

                    let place = self.translate_place_smir(place);
                    let target = GcCow::new(place);
                    let mutbl = translate_mutbl_smir(bkind.to_mutable_lossy());

                    let ptr_ty = PtrType::Ref { mutbl, pointee };

                    ValueExpr::AddrOf { target, ptr_ty }
                }
                smir::Rvalue::AddressOf(_mutbl, place) => {
                    let place = self.translate_place_smir(place);
                    let target = GcCow::new(place);

                    let ptr_ty = PtrType::Raw;

                    ValueExpr::AddrOf { target, ptr_ty }
                }
                smir::Rvalue::Aggregate(agg, operands) => {
                    let ty = rv.ty(&self.locals_smir).unwrap();
                    let ty = self.translate_ty_smir(ty);
                    match ty {
                        Type::Union { .. } => {
                            let smir::AggregateKind::Adt(_, _, _, _, Some(field_idx)) = agg else {
                                panic!()
                            };
                            assert_eq!(operands.len(), 1);
                            let expr = self.translate_operand_smir(&operands[0]);
                            ValueExpr::Union {
                                field: (*field_idx).into(),
                                expr: GcCow::new(expr),
                                union_ty: ty,
                            }
                        }
                        Type::Tuple { .. } | Type::Array { .. } => {
                            let ops: List<_> =
                                operands.iter().map(|x| self.translate_operand_smir(x)).collect();
                            ValueExpr::Tuple(ops, ty)
                        }
                        Type::Enum { variants, .. } => {
                            let smir::AggregateKind::Adt(_, variant_idx, _, _, _) = agg else {
                                panic!()
                            };
                            let discriminant = self.discriminant_for_variant_smir(
                                rv.ty(&self.locals_smir).unwrap(),
                                *variant_idx,
                            );
                            let ops: List<_> =
                                operands.iter().map(|x| self.translate_operand_smir(x)).collect();

                            // We represent the multiple fields of an enum variant as a MiniRust tuple.
                            let data = GcCow::new(ValueExpr::Tuple(
                                ops,
                                variants.get(discriminant).unwrap().ty,
                            ));
                            ValueExpr::Variant { discriminant, data, enum_ty: ty }
                        }
                        _ => panic!("invalid aggregate type!"),
                    }
                }
                smir::Rvalue::CopyForDeref(place) =>
                    ValueExpr::Load { source: GcCow::new(self.translate_place_smir(place)) },
                smir::Rvalue::Len(place) => {
                    // as slices are unsupported as of now, we only need to care for arrays.
                    let ty = place.ty(&self.locals_smir).unwrap();
                    let Type::Array { elem: _, count } = self.translate_ty_smir(ty) else {
                        panic!()
                    };
                    ValueExpr::Constant(Constant::Int(count), <usize>::get_type())
                }
                smir::Rvalue::Discriminant(place) =>
                    ValueExpr::GetDiscriminant {
                        place: GcCow::new(self.translate_place_smir(place)),
                    },
                smir::Rvalue::Cast(smir::CastKind::IntToInt, operand, ty) => {
                    let operand_ty = self.translate_ty_smir(operand.ty(&self.locals_smir).unwrap());
                    let operand = self.translate_operand_smir(operand);
                    let Type::Int(int_ty) = self.translate_ty_smir(*ty) else {
                        panic!("attempting to IntToInt-Cast to non-int type!");
                    };

                    let unop = match operand_ty {
                        Type::Int(_) => UnOp::Int(UnOpInt::Cast, int_ty),
                        Type::Bool => UnOp::Bool(UnOpBool::IntCast(int_ty)),
                        _ => panic!("Attempting to cast non-int or boolean type to int!"),
                    };
                    ValueExpr::UnOp { operator: unop, operand: GcCow::new(operand) }
                }
                smir::Rvalue::Cast(smir::CastKind::PointerExposeAddress, operand, _) => {
                    let operand = self.translate_operand_smir(operand);
                    let expose = Statement::Expose { value: operand };
                    let addr = build::ptr_addr(operand);

                    return Some((vec![expose], addr));
                }
                smir::Rvalue::Cast(smir::CastKind::PointerFromExposedAddress, operand, ty) => {
                    // TODO untested so far! (Can't test because of `predict`)
                    let operand = self.translate_operand_smir(operand);
                    let Type::Ptr(ptr_ty) = self.translate_ty_smir(*ty) else { panic!() };

                    ValueExpr::UnOp {
                        operator: UnOp::PtrFromExposed(ptr_ty),
                        operand: GcCow::new(operand),
                    }
                }
                smir::Rvalue::Cast(smir::CastKind::PtrToPtr, operand, ty) => {
                    let operand = self.translate_operand_smir(operand);
                    let Type::Ptr(ptr_ty) = self.translate_ty_smir(*ty) else { panic!() };

                    ValueExpr::UnOp {
                        operator: UnOp::Transmute(Type::Ptr(ptr_ty)),
                        operand: GcCow::new(operand),
                    }
                }
                smir::Rvalue::Repeat(op, c) => {
                    let c = c.eval_target_usize().unwrap();
                    let c = Int::from(c);

                    let elem_ty = self.translate_ty_smir(op.ty(&self.locals_smir).unwrap());
                    let op = self.translate_operand_smir(op);

                    let ty = Type::Array { elem: GcCow::new(elem_ty), count: c };

                    let ls = list![op; c];
                    ValueExpr::Tuple(ls, ty)
                }
                smir::Rvalue::Cast(
                    smir::CastKind::PointerCoercion(smir::PointerCoercion::ReifyFnPointer),
                    func,
                    _,
                ) => {
                    let smir::Operand::Constant(f1) = func else { panic!() };
                    let smir::TyKind::RigidTy(smir::RigidTy::FnDef(f, substs_ref)) = f1.ty().kind()
                    else {
                        panic!()
                    };
                    let instance = smir::Instance::resolve(f, &substs_ref).unwrap();

                    build::fn_ptr(self.cx.get_fn_name_smir(instance).0.get_internal())
                }
                smir::Rvalue::NullaryOp(smir::NullOp::DebugAssertions, _ty) => {
                    // Like Miri, since we are able to detect language UB ourselves we can disable these checks.
                    build::const_bool(false)
                }
                x => {
                    dbg!(x);
                    todo!()
                }
            },
        ))
    }

    pub fn translate_operand(&mut self, operand: &rs::Operand<'tcx>) -> ValueExpr {
        self.translate_operand_smir(&smir::stable(operand))
    }

    pub fn translate_operand_smir(&mut self, operand: &smir::Operand) -> ValueExpr {
        match operand {
            smir::Operand::Constant(c) => self.translate_const_smir(&c.literal),
            smir::Operand::Copy(place) =>
                ValueExpr::Load { source: GcCow::new(self.translate_place_smir(place)) },
            smir::Operand::Move(place) =>
                ValueExpr::Load { source: GcCow::new(self.translate_place_smir(place)) },
        }
    }

    pub fn translate_place(&mut self, place: &rs::Place<'tcx>) -> PlaceExpr {
        self.translate_place_smir(&smir::stable(place))
    }

    pub fn translate_place_smir(&mut self, place: &smir::Place) -> PlaceExpr {
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

                        let ty = self.translate_ty_smir(this_ty);

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
                            self.discriminant_for_variant_smir(this_ty, *variant_idx);
                        PlaceExpr::Downcast { root, discriminant }
                    }
                    x => todo!("{:?}", x),
                };
                (this_expr, this_ty)
            });
        expr
    }
}
