use crate::*;

/// Translate an rvalue -- could generate a bunch of helper statements.
pub fn translate_rvalue<'cx, 'tcx>(
    rv: &rs::Rvalue<'tcx>,
    fcx: &mut FnCtxt<'cx, 'tcx>,
) -> Option<(Vec<Statement>, ValueExpr)> {
    Some((vec![], match rv {
        rs::Rvalue::Use(operand) => translate_operand(operand, fcx),
        rs::Rvalue::CheckedBinaryOp(bin_op, box (l, r))
        | rs::Rvalue::BinaryOp(bin_op, box (l, r)) => {
            let lty = l.ty(&fcx.body, fcx.cx.tcx);
            let rty = r.ty(&fcx.body, fcx.cx.tcx);

            assert_eq!(lty, rty);

            let l = translate_operand(l, fcx);
            let r = translate_operand(r, fcx);

            let l = GcCow::new(l);
            let r = GcCow::new(r);

            use rs::BinOp::*;
            let op = if *bin_op == Offset {
                BinOp::PtrOffset { inbounds: true }
            } else {
                // everything else right-now is a int op!

                let op = |x| {
                    let Type::Int(int_ty) = fcx.translate_ty(lty) else {
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

            ValueExpr::BinOp {
                operator: op,
                left: l,
                right: r,
            }
        }
        rs::Rvalue::UnaryOp(unop, operand) => match unop {
            rs::UnOp::Neg => {
                let ty = operand.ty(&fcx.body, fcx.cx.tcx);
                let ty = fcx.translate_ty(ty);
                let Type::Int(int_ty) = ty else {
                        panic!("Neg operation with non-int type!");
                    };

                let operand = translate_operand(operand, fcx);

                ValueExpr::UnOp {
                    operator: UnOp::Int(UnOpInt::Neg, int_ty),
                    operand: GcCow::new(operand),
                }
            }
            rs::UnOp::Not => {
                let ty = operand.ty(&fcx.body, fcx.cx.tcx);
                let ty = fcx.translate_ty(ty);
                let Type::Bool = ty else {
                        panic!("Not operation with non-boolean type!");
                    };

                let operand = translate_operand(operand, fcx);

                ValueExpr::UnOp {
                    operator: UnOp::Bool(UnOpBool::Not),
                    operand: GcCow::new(operand)
                }
            }
        },
        rs::Rvalue::Ref(_, bkind, place) => {
            let ty = place.ty(&fcx.body, fcx.cx.tcx).ty;
            let pointee = fcx.layout_of(ty);

            let place = translate_place(place, fcx);
            let target = GcCow::new(place);
            let mutbl = translate_mutbl(bkind.to_mutbl_lossy());

            let ptr_ty = PtrType::Ref { mutbl, pointee };

            ValueExpr::AddrOf { target, ptr_ty }
        }
        rs::Rvalue::AddressOf(_mutbl, place) => {
            let place = translate_place(place, fcx);
            let target = GcCow::new(place);

            let ptr_ty = PtrType::Raw;

            ValueExpr::AddrOf { target, ptr_ty }
        }
        rs::Rvalue::Aggregate(box agg, operands) => {
            let ty = rv.ty(&fcx.body, fcx.cx.tcx);
            let ty = fcx.translate_ty(ty);
            match ty {
                Type::Union { .. } => {
                    let rs::AggregateKind::Adt(_, _, _, _, Some(field_idx)) = agg else { panic!() };
                    assert_eq!(operands.len(), 1);
                    let expr = translate_operand(&operands[rs::FieldIdx::from_u32(0)], fcx);
                    ValueExpr::Union {
                        field: field_idx.index().into(),
                        expr: GcCow::new(expr),
                        union_ty: ty,
                    }
                }
                Type::Tuple { .. } | Type::Array { .. } => {
                    let ops: List<_> = operands.iter().map(|x| translate_operand(x, fcx)).collect();
                    ValueExpr::Tuple(ops, ty)
                }
                Type::Enum { variants, .. } => {
                    let rs::AggregateKind::Adt(_, variant_idx, _, _, _) = agg else { panic!() };
                    let discriminant = fcx.discriminant_for_variant(rv.ty(&fcx.body, fcx.cx.tcx), *variant_idx);
                    let ops: List<_> = operands.iter().map(|x| translate_operand(x, fcx)).collect();

                    // We represent the multiple fields of an enum variant as a MiniRust tuple.
                    let data = GcCow::new(ValueExpr::Tuple(ops, variants.get(discriminant).unwrap().ty));
                    ValueExpr::Variant { discriminant, data, enum_ty: ty }
                }
                _ => panic!("invalid aggregate type!"),
            }
        }
        rs::Rvalue::CopyForDeref(place) => ValueExpr::Load {
            source: GcCow::new(translate_place(place, fcx)),
        },
        rs::Rvalue::Len(place) => {
            // as slices are unsupported as of now, we only need to care for arrays.
            let ty = place.ty(&fcx.body, fcx.cx.tcx).ty;
            let Type::Array { elem: _, count } = fcx.translate_ty(ty) else { panic!() };
            ValueExpr::Constant(Constant::Int(count), <usize>::get_type())
        },
        rs::Rvalue::Discriminant(place) => ValueExpr::GetDiscriminant {
            place: GcCow::new(translate_place(place, fcx))
        },
        rs::Rvalue::Cast(rs::CastKind::IntToInt, operand, ty) => {
            let operand_ty = fcx.translate_ty(operand.ty(&fcx.body.local_decls, fcx.cx.tcx));
            let operand = translate_operand(operand, fcx);
            let Type::Int(int_ty) = fcx.translate_ty(*ty) else {
                panic!("attempting to IntToInt-Cast to non-int type!");
            };

            let unop = match operand_ty {
                Type::Int(_) => UnOp::Int(UnOpInt::Cast, int_ty),
                Type::Bool => UnOp::Bool(UnOpBool::IntCast(int_ty)),
                _ => panic!("Attempting to cast non-int or boolean type to int!")
            };
            ValueExpr::UnOp {
                operator: unop,
                operand: GcCow::new(operand),
            }
        },
        rs::Rvalue::Cast(rs::CastKind::PointerExposeAddress, operand, _) => {
            let operand = translate_operand(operand, fcx);
            let expose = Statement::Expose { value: operand };
            let addr = build::ptr_addr(operand);

            return Some((
                vec![expose],
                addr
            ));  
        }
        rs::Rvalue::Cast(rs::CastKind::PointerFromExposedAddress, operand, ty) => {
            // TODO untested so far! (Can't test because of `predict`)
            let operand = translate_operand(operand, fcx);
            let Type::Ptr(ptr_ty) = fcx.translate_ty(*ty) else { panic!() };

            ValueExpr::UnOp {
                operator: UnOp::PtrFromExposed(ptr_ty),
                operand: GcCow::new(operand),
            }
        }
        rs::Rvalue::Cast(rs::CastKind::PtrToPtr, operand, ty) => {
            let operand = translate_operand(operand, fcx);
            let Type::Ptr(ptr_ty) = fcx.translate_ty(*ty) else { panic!() };

            ValueExpr::UnOp {
                operator: UnOp::Transmute(Type::Ptr(ptr_ty)),
                operand: GcCow::new(operand),
            }
        }
        rs::Rvalue::Repeat(op, c) => {
            let c = c.try_eval_target_usize(fcx.cx.tcx, rs::ParamEnv::reveal_all()).unwrap();
            let c = Int::from(c);

            let elem_ty = fcx.translate_ty(op.ty(&fcx.body, fcx.cx.tcx));
            let op = translate_operand(op, fcx);

            let ty = Type::Array {
                elem: GcCow::new(elem_ty),
                count: c,
            };

            let ls = list![op; c];
            ValueExpr::Tuple(ls, ty)
        }
        rs::Rvalue::Cast(rs::CastKind::PointerCoercion(rs::adjustment::PointerCoercion::ReifyFnPointer), func, _) => {
            let rs::Operand::Constant(box f1) = func else { panic!() };
            let rs::mir::Const::Val(_, f2) = f1.const_ else { panic!() };
            let rs::TyKind::FnDef(f, substs_ref) = f2.kind() else { panic!() };
            let instance = rs::Instance::resolve(
                fcx.cx.tcx,
                rs::ParamEnv::reveal_all(),
                *f,
                substs_ref,
            ).unwrap().unwrap();

            build::fn_ptr(
                fcx.cx.get_fn_name(instance).0.get_internal()
            )
        }
        rs::Rvalue::NullaryOp(rs::NullOp::DebugAssertions, _ty) => {
            build::const_bool(false)
        }
        x => {
            dbg!(x);
            todo!()
        }
    }))
}

pub fn translate_operand<'cx, 'tcx>(
    operand: &rs::Operand<'tcx>,
    fcx: &mut FnCtxt<'cx, 'tcx>,
) -> ValueExpr {
    match operand {
        rs::Operand::Constant(box c) => translate_const(c, fcx),
        rs::Operand::Copy(place) => ValueExpr::Load {
            source: GcCow::new(translate_place(place, fcx)),
        },
        rs::Operand::Move(place) => ValueExpr::Load {
            source: GcCow::new(translate_place(place, fcx)),
        },
    }
}

pub fn translate_place<'cx, 'tcx>(
    place: &rs::Place<'tcx>,
    fcx: &mut FnCtxt<'cx, 'tcx>,
) -> PlaceExpr {
    let mut expr = PlaceExpr::Local(fcx.local_name_map[&place.local]);
    for (i, proj) in place.projection.iter().enumerate() {
        match proj {
            rs::ProjectionElem::Field(f, _ty) => {
                let f = f.index();
                let indirected = GcCow::new(expr);
                expr = PlaceExpr::Field {
                    root: indirected,
                    field: f.into(),
                };
            }
            rs::ProjectionElem::Deref => {
                let x = GcCow::new(expr);
                let x = ValueExpr::Load {
                    source: x,
                };
                let x = GcCow::new(x);

                let ty = rs::Place::ty_from(
                    place.local,
                    &place.projection[..(i + 1)],
                    &fcx.body,
                    fcx.cx.tcx,
                )
                .ty;
                let ty = fcx.translate_ty(ty);

                expr = PlaceExpr::Deref { operand: x, ty };
            }
            rs::ProjectionElem::Index(loc) => {
                let i = PlaceExpr::Local(fcx.local_name_map[&loc]);
                let i = GcCow::new(i);
                let i = ValueExpr::Load {
                    source: i,
                };
                let i = GcCow::new(i);
                let root = GcCow::new(expr);
                expr = PlaceExpr::Index { root, index: i };
            }
            rs::ProjectionElem::Downcast(_variant_name, variant_idx) => {
                let root = GcCow::new(expr);
                let ty = rs::Place::ty_from(
                    place.local,
                    &place.projection[..(i + 1)],
                    &fcx.body,
                    fcx.cx.tcx,
                ).ty;
                let discriminant = fcx.discriminant_for_variant(ty, variant_idx);
                expr = PlaceExpr::Downcast { root, discriminant };
            }
            x => todo!("{:?}", x),
        }
    }
    expr
}
