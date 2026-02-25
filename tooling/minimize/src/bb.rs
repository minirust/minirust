use rustc_middle::ty::layout::FnAbiOf;

use crate::*;

// Some Rust features are not supported, and are ignored by `minimize`.
// Those can be found by grepping "IGNORED".

/// A MIR statement becomes either a MiniRust statement or an intrinsic with some arguments, which
/// then starts a new basic block.
enum StatementResult {
    Statement(Statement),
    Intrinsic { intrinsic: IntrinsicOp, destination: PlaceExpr, arguments: List<ValueExpr> },
}

/// A MIR terminator becomes a MiniRust terminator, possibly preceded by a list
/// of statements to execute at the end of the previous basic block.
struct TerminatorResult {
    stmts: List<Statement>,
    terminator: Terminator,
}

impl<'cx, 'tcx> FnCtxt<'cx, 'tcx> {
    /// Translate the given basic block and insert it into `self` with the given name.
    /// May insert more than one block because some MIR statements turn into MiniRust terminators.
    pub fn translate_bb(&mut self, name: BbName, bb: &rs::BasicBlockData<'tcx>) {
        let mut cur_block_name = name;
        let mut cur_block_statements = List::new();
        // Translate the block kind.
        let block_kind = if bb.is_cleanup { BbKind::Cleanup } else { BbKind::Regular };

        for stmt in bb.statements.iter() {
            match self.translate_stmt(stmt) {
                StatementResult::Statement(stmt) => {
                    cur_block_statements.push(stmt);
                }
                StatementResult::Intrinsic { intrinsic, destination, arguments } => {
                    // Generate a fresh bb name.
                    let next_bb = self.fresh_bb_name();
                    // End the current block by jumping to the next one.
                    let terminator = Terminator::Intrinsic {
                        intrinsic,
                        arguments,
                        ret: destination,
                        next_block: Some(next_bb),
                    };
                    let cur_block = BasicBlock {
                        statements: cur_block_statements,
                        terminator,
                        kind: block_kind,
                    };
                    let old = self.blocks.insert(cur_block_name, cur_block);
                    assert!(old.is_none()); // make sure we do not overwrite a bb
                    // Go on building the next block.
                    cur_block_name = next_bb;
                    cur_block_statements = List::new();
                }
            }
        }
        let TerminatorResult { stmts, terminator } = self.translate_terminator(bb);
        for stmt in stmts.iter() {
            cur_block_statements.push(stmt);
        }
        let cur_block =
            BasicBlock { statements: cur_block_statements, terminator, kind: block_kind };
        let old = self.blocks.insert(cur_block_name, cur_block);
        assert!(old.is_none()); // make sure we do not overwrite a bb
    }

    fn translate_stmt(&mut self, stmt: &rs::Statement<'tcx>) -> StatementResult {
        let span = stmt.source_info.span;
        StatementResult::Statement(match &stmt.kind {
            rs::StatementKind::Assign(box (place, rval)) => {
                let destination = self.translate_place(place, span);
                // Some things that MIR handles as rvalues are non-deterministic,
                // so MiniRust treats them differently.
                match rval {
                    rs::Rvalue::Cast(rs::CastKind::PointerExposeProvenance, operand, _) => {
                        let operand = self.translate_operand(operand, span);
                        return StatementResult::Intrinsic {
                            intrinsic: IntrinsicOp::PointerExposeProvenance,
                            destination,
                            arguments: list![operand],
                        };
                    }
                    rs::Rvalue::Cast(rs::CastKind::PointerWithExposedProvenance, operand, _) => {
                        // TODO untested so far! (Can't test because of `predict`)
                        let operand = self.translate_operand(operand, span);
                        return StatementResult::Intrinsic {
                            intrinsic: IntrinsicOp::PointerWithExposedProvenance,
                            destination,
                            arguments: list![operand],
                        };
                    }
                    _ => {}
                }
                let source = self.translate_rvalue(rval, span);
                Statement::Assign { destination, source }
            }
            rs::StatementKind::StorageLive(local) =>
                Statement::StorageLive(self.local_name_map[&local]),
            rs::StatementKind::StorageDead(local) =>
                Statement::StorageDead(self.local_name_map[&local]),
            rs::StatementKind::Retag(kind, place) => {
                let place = self.translate_place(place, span);
                let fn_entry = matches!(kind, rs::RetagKind::FnEntry);
                Statement::Validate { place, fn_entry }
            }
            rs::StatementKind::Deinit(place) => {
                let place = self.translate_place(place, span);
                Statement::Deinit { place }
            }
            rs::StatementKind::SetDiscriminant { place, variant_index } => {
                let place_ty =
                    rs::Place::ty_from(place.local, place.projection, &self.body, self.tcx).ty;
                let discriminant = self.discriminant_for_variant(place_ty, *variant_index, span);
                Statement::SetDiscriminant {
                    destination: self.translate_place(place, span),
                    value: discriminant,
                }
            }
            rs::StatementKind::Intrinsic(box intrinsic) => {
                match intrinsic {
                    rs::NonDivergingIntrinsic::Assume(op) => {
                        let op = self.translate_operand(op, span);
                        // Doesn't return anything, get us a dummy place.
                        let destination = build::unit_place();
                        return StatementResult::Intrinsic {
                            intrinsic: IntrinsicOp::Assume,
                            destination,
                            arguments: list![op],
                        };
                    }
                    rs::NonDivergingIntrinsic::CopyNonOverlapping(_) =>
                        rs::span_bug!(span, "NonDivergingIntrinsic not supported: {intrinsic:?}"),
                }
            }
            rs::StatementKind::PlaceMention(place) => {
                let place = self.translate_place(place, span);
                Statement::PlaceMention(place)
            }

            rs::StatementKind::FakeRead(_)
            | rs::StatementKind::AscribeUserType(_, _)
            | rs::StatementKind::Coverage(_)
            | rs::StatementKind::ConstEvalCounter
            | rs::StatementKind::Nop
            | rs::StatementKind::BackwardIncompatibleDropHint { .. } => {
                rs::span_bug!(span, "Statement not supported: {:?}", stmt.kind);
            }
        })
    }

    fn translate_terminator(&mut self, bb: &rs::BasicBlockData<'tcx>) -> TerminatorResult {
        let terminator = bb.terminator();
        let span = terminator.source_info.span;
        let terminator = match &terminator.kind {
            rs::TerminatorKind::Return => Terminator::Return,
            rs::TerminatorKind::Goto { target } => Terminator::Goto(self.bb_name_map[&target]),
            rs::TerminatorKind::Call { func, target, destination, args, unwind, .. } =>
                return self.translate_call(func, args, destination, target, span, *unwind, bb),
            rs::TerminatorKind::SwitchInt { discr, targets } => {
                let ty = discr.ty(&self.body, self.tcx);
                let ty = self.translate_ty(ty, span);

                let discr_op = self.translate_operand(discr, span);
                let (value, int_ty) = match ty {
                    Type::Bool => {
                        // If the value is a boolean we need to cast it to an integer first as MiniRust switch only operates on ints.
                        let Type::Int(u8_inttype) = <u8>::get_type() else { unreachable!() };
                        (
                            ValueExpr::UnOp {
                                operator: UnOp::Cast(CastOp::Transmute(Type::Int(u8_inttype))),
                                operand: GcCow::new(discr_op),
                            },
                            u8_inttype,
                        )
                    }
                    Type::Int(ity) => (discr_op, ity),
                    // FIXME: add support for switching on `char`
                    _ =>
                        rs::span_bug!(
                            span,
                            "SwitchInt terminator currently only supports int and bool."
                        ),
                };

                let cases = targets
                    .iter()
                    .map(|(value, target)| {
                        (int_from_bits(value, int_ty), self.bb_name_map[&target])
                    })
                    .collect();

                let fallback_block = targets.otherwise();
                let fallback = self.bb_name_map[&fallback_block];

                Terminator::Switch { value, cases, fallback }
            }
            rs::TerminatorKind::Unreachable => Terminator::Unreachable,
            rs::TerminatorKind::Assert { cond, expected, target, unwind, .. } => {
                let mut condition = self.translate_operand(cond, span);
                // Check equality of `condition` and `expected`.
                // We do this by inverting `condition` if `expected` is false
                // and then checking if `condition` is true.
                if !expected {
                    condition = build::not(condition);
                }

                // Create panic block in case of `expected != condition`
                let terminator = self.translate_panic(*unwind, bb);
                let panic_block_name = self.fresh_bb_name();
                let block_kind = if bb.is_cleanup { BbKind::Cleanup } else { BbKind::Regular };
                let panic_block = BasicBlock { statements: list![], terminator, kind: block_kind };
                self.blocks.try_insert(panic_block_name, panic_block).unwrap();

                let next_block = self.bb_name_map[target];
                Terminator::Switch {
                    value: build::bool_to_int::<u8>(condition),
                    cases: [(Int::from(1), next_block)].into_iter().collect(),
                    fallback: panic_block_name,
                }
            }
            rs::TerminatorKind::Drop { place, target, .. } => {
                let ty = place.ty(&self.body, self.tcx).ty;
                let place = self.translate_place(place, span);
                let (drop_fn, ptr_to_drop) = match ty.kind() {
                    // For trait objects we must first fetch the drop function dynamically
                    rs::TyKind::Dynamic(..) => {
                        let Type::TraitObject(trait_name) = self.translate_ty(ty, span) else {
                            rs::span_bug!(
                                span,
                                "translate_ty for TyKind::Dynamic didn't give a Type::TraitObject"
                            );
                        };
                        // Compute the wide pointer pointing to `dyn Trait`.
                        let ptr = build::addr_of(
                            place,
                            build::raw_ptr_ty(PointerMetaKind::VTablePointer(trait_name)),
                        );
                        // Fetch the drop function from the vtable.
                        let drop_fn = build::vtable_method_lookup(
                            build::get_metadata(ptr),
                            TraitMethodName(Name::from_internal(
                                rs::COMMON_VTABLE_ENTRIES_DROPINPLACE as _,
                            )),
                        );
                        // Only the thin part of the pointer gets passed to the drop function.
                        (drop_fn, build::get_thin_pointer(ptr))
                    }
                    // For other types we can just get the drop instance statically
                    _ => {
                        let drop_in_place_fn = rs::Instance::resolve_drop_in_place(self.tcx, ty);
                        let ptr_to_drop = build::addr_of(place, build::raw_void_ptr_ty());
                        let drop_fn = build::fn_ptr(self.cx.get_fn_name(drop_in_place_fn));
                        (drop_fn, ptr_to_drop)
                    }
                };

                Terminator::Call {
                    callee: drop_fn,
                    calling_convention: CallingConvention::Rust,
                    arguments: list![ArgumentExpr::ByValue(ptr_to_drop)],
                    ret: unit_place(),
                    next_block: Some(self.bb_name_map[&target]),
                    unwind_block: None,
                }
            }
            rs::TerminatorKind::UnwindResume => Terminator::ResumeUnwind,

            rs::TerminatorKind::UnwindTerminate(_)
            | rs::TerminatorKind::TailCall { .. }
            | rs::TerminatorKind::Yield { .. }
            | rs::TerminatorKind::CoroutineDrop
            | rs::TerminatorKind::FalseEdge { .. }
            | rs::TerminatorKind::FalseUnwind { .. }
            | rs::TerminatorKind::InlineAsm { .. } => {
                rs::span_bug!(span, "Terminator not supported: {:?}", terminator.kind);
            }
        };

        TerminatorResult { terminator, stmts: List::new() }
    }

    /// Translate Rust's `catch_unwind` intrinsic to MiniRust.
    ///
    /// This translates `ret = catch_unwind(try_fn, data_ptr, catch_fn)` into the following blocks:
    /// ```
    /// // try block
    /// bb0:
    ///     let ret_tmp = &raw mut ret;
    ///     let try_fn_tmp = try_fn;
    ///     let data_tmp = data_ptr;
    ///     let catch_fn_tmp = catch_fn;
    ///     try_fn_tmp(data_tmp) -> [return: bb1, unwind: bb2]
    /// // return block
    /// bb1:
    ///     *ret_tmp = 0;
    ///     goTo -> bb5
    /// // get_payload block
    /// bb2 (Catch):
    ///     let unwind_payload_tmp = get_unwind_payload() -> return: bb3
    /// // catch block
    /// bb3 (Catch):
    ///     *ret_tmp = 1;
    ///     catch_fn_tmp(data_tmp, unwind_payload_tmp) -> return: bb4
    /// // stop_unwind block
    /// bb4 (Catch):
    ///     StopUnwind -> bb5
    /// // target (next block of the program)
    /// bb5:
    ///     // The program continues
    ///
    /// ```
    /// This function returns a `goTo` terminator pointing to the try block.
    fn translate_catch_unwind(
        &mut self,
        args: List<ValueExpr>,
        destination: PlaceExpr,
        target: BbName,
    ) -> Terminator {
        let try_fn = args.index_at(0);
        let data = args.index_at(1);
        let catch_fn = args.index_at(2);

        // the names of the new blocks
        let try_bb_name = self.fresh_bb_name();
        let return_bb_name = self.fresh_bb_name();
        let get_payload_bb_name = self.fresh_bb_name();
        let catch_bb_name = self.fresh_bb_name();
        let stop_unwind_bb_name = self.fresh_bb_name();

        // generate temporary locals to store the arguments, `ret`, and `payload`
        let ret_tmp = self.fresh_local_name();
        self.locals.insert(ret_tmp, Type::Ptr(PtrType::Raw { meta_kind: PointerMetaKind::None }));
        let try_fn_tmp = self.fresh_local_name();
        self.locals.insert(try_fn_tmp, Type::Ptr(PtrType::FnPtr));
        let data_tmp = self.fresh_local_name();
        self.locals.insert(data_tmp, Type::Ptr(PtrType::Raw { meta_kind: PointerMetaKind::None }));
        let catch_fn_tmp = self.fresh_local_name();
        self.locals.insert(catch_fn_tmp, Type::Ptr(PtrType::FnPtr));
        let unwind_payload_tmp = self.fresh_local_name();
        self.locals.insert(
            unwind_payload_tmp,
            Type::Ptr(PtrType::Raw { meta_kind: PointerMetaKind::None }),
        );

        // generate the try block
        let try_bb = BasicBlock {
            statements: list![
                Statement::StorageLive(ret_tmp),
                Statement::StorageLive(try_fn_tmp),
                Statement::StorageLive(data_tmp),
                Statement::StorageLive(catch_fn_tmp),
                Statement::Assign {
                    destination: PlaceExpr::Local(ret_tmp),
                    source: ValueExpr::AddrOf {
                        target: GcCow::new(destination),
                        ptr_ty: PtrType::Raw { meta_kind: PointerMetaKind::None }
                    }
                },
                Statement::Assign { destination: PlaceExpr::Local(try_fn_tmp), source: try_fn },
                Statement::Assign { destination: PlaceExpr::Local(data_tmp), source: data },
                Statement::Assign { destination: PlaceExpr::Local(catch_fn_tmp), source: catch_fn }
            ],
            terminator: Terminator::Call {
                callee: ValueExpr::Load { source: GcCow::new(PlaceExpr::Local(try_fn_tmp)) },
                calling_convention: CallingConvention::Rust,
                arguments: list![build::by_value(ValueExpr::Load {
                    source: GcCow::new(PlaceExpr::Local(data_tmp))
                })],
                ret: unit_place(),
                next_block: Some(return_bb_name),
                unwind_block: Some(get_payload_bb_name),
            },
            kind: BbKind::Regular,
        };
        self.blocks.insert(try_bb_name, try_bb);

        // generate the return block
        let return_bb = BasicBlock {
            statements: list![
                Statement::Assign {
                    destination: PlaceExpr::Deref {
                        ty: Type::Int(IntType::I32),
                        operand: GcCow::new(ValueExpr::Load {
                            source: GcCow::new(PlaceExpr::Local(ret_tmp))
                        })
                    },
                    source: build::const_int(0)
                },
                Statement::StorageDead(ret_tmp),
                Statement::StorageDead(try_fn_tmp),
                Statement::StorageDead(data_tmp),
                Statement::StorageDead(catch_fn_tmp)
            ],
            terminator: Terminator::Goto(target),
            kind: BbKind::Regular,
        };
        self.blocks.insert(return_bb_name, return_bb);

        // generate the get_payload block
        let get_payload_bb = BasicBlock {
            statements: list![Statement::StorageLive(unwind_payload_tmp)],
            terminator: Terminator::Intrinsic {
                intrinsic: IntrinsicOp::GetUnwindPayload,
                arguments: list![],
                ret: PlaceExpr::Local(unwind_payload_tmp),
                next_block: Some(catch_bb_name),
            },
            kind: BbKind::Catch,
        };
        self.blocks.insert(get_payload_bb_name, get_payload_bb);

        // generate the catch block
        let catch_bb = BasicBlock {
            statements: list![Statement::Assign {
                destination: PlaceExpr::Deref {
                    ty: Type::Int(IntType::I32),
                    operand: GcCow::new(ValueExpr::Load {
                        source: GcCow::new(PlaceExpr::Local(ret_tmp))
                    })
                },
                source: build::const_int(1)
            }],
            terminator: Terminator::Call {
                callee: ValueExpr::Load { source: GcCow::new(PlaceExpr::Local(catch_fn_tmp)) },
                calling_convention: CallingConvention::Rust,
                arguments: list![
                    build::by_value(ValueExpr::Load {
                        source: GcCow::new(PlaceExpr::Local(data_tmp))
                    }),
                    build::by_value(ValueExpr::Load {
                        source: GcCow::new(PlaceExpr::Local(unwind_payload_tmp))
                    })
                ],
                ret: unit_place(),
                next_block: Some(stop_unwind_bb_name),
                unwind_block: None,
            },
            kind: BbKind::Catch,
        };
        self.blocks.insert(catch_bb_name, catch_bb);

        // generate the stop_unwind block
        let stop_unwind_bb = BasicBlock {
            statements: list![
                Statement::StorageDead(ret_tmp),
                Statement::StorageDead(try_fn_tmp),
                Statement::StorageDead(data_tmp),
                Statement::StorageDead(catch_fn_tmp)
            ],
            terminator: Terminator::StopUnwind(target),
            kind: BbKind::Catch,
        };
        self.blocks.insert(stop_unwind_bb_name, stop_unwind_bb);
        Terminator::Goto(try_bb_name)
    }

    fn translate_rs_intrinsic(
        &mut self,
        intrinsic: rs::Instance<'tcx>,
        args: &[rs::Spanned<rs::Operand<'tcx>>],
        destination: &rs::Place<'tcx>,
        target: &Option<rs::BasicBlock>,
        span: rs::Span,
    ) -> TerminatorResult {
        let intrinsic_name = self.tcx.item_name(intrinsic.def_id());
        match intrinsic_name {
            rs::sym::assert_inhabited
            | rs::sym::assert_zero_valid
            | rs::sym::assert_mem_uninitialized_valid => {
                let ty = intrinsic.args.type_at(0);
                let requirement =
                    rs::layout::ValidityRequirement::from_intrinsic(intrinsic_name).unwrap();
                let should_panic = !self
                    .tcx
                    .check_validity_requirement((requirement, self.typing_env().as_query_input(ty)))
                    .unwrap();

                let terminator = if should_panic {
                    Terminator::Intrinsic {
                        intrinsic: IntrinsicOp::Abort,
                        arguments: list![],
                        ret: unit_place(),
                        next_block: None,
                    }
                } else {
                    Terminator::Goto(self.bb_name_map[&target.unwrap()])
                };
                return TerminatorResult { terminator, stmts: List::new() };
            }
            rs::sym::raw_eq =>
                return TerminatorResult {
                    stmts: List::new(),
                    terminator: Terminator::Intrinsic {
                        intrinsic: IntrinsicOp::RawEq,
                        arguments: args
                            .iter()
                            .map(|x| self.translate_operand(&x.node, x.span))
                            .collect(),
                        ret: self.translate_place(&destination, span),
                        next_block: target.as_ref().map(|t| self.bb_name_map[t]),
                    },
                },
            rs::sym::arith_offset => {
                let lty = args[0].node.ty(&self.body, self.tcx);
                let rty = args[1].node.ty(&self.body, self.tcx);

                let l = self.translate_operand(&args[0].node, span);
                let r = self.translate_operand(&args[1].node, span);
                let destination = self.translate_place(&destination, span);

                let pointee = lty.builtin_deref(true).unwrap();
                let pointee = self.rs_layout_of(pointee);
                assert!(pointee.is_sized());
                let size = Int::from(pointee.size.bytes());
                let size = ValueExpr::Constant(Constant::Int(size), self.translate_ty(rty, span));
                let offset_bytes = build::mul_unchecked(r, size);

                let val = build::ptr_offset(l, offset_bytes, build::InBounds::No);

                let stmt = Statement::Assign { destination, source: val };
                let terminator = Terminator::Goto(self.bb_name_map[&target.unwrap()]);

                return TerminatorResult { stmts: list![stmt], terminator };
            }
            rs::sym::ptr_offset_from | rs::sym::ptr_offset_from_unsigned => {
                let unsigned = intrinsic_name == rs::sym::ptr_offset_from_unsigned;
                let lty = args[0].node.ty(&self.body, self.tcx);

                let l = self.translate_operand(&args[0].node, span);
                let r = self.translate_operand(&args[1].node, span);
                let destination = self.translate_place(&destination, span);

                // Compute distance in bytes.
                let offset_bytes = ValueExpr::BinOp {
                    operator: BinOp::PtrOffsetFrom { inbounds: false, nonneg: unsigned },
                    left: GcCow::new(l),
                    right: GcCow::new(r),
                };
                // Divide by the size.
                let pointee = lty.builtin_deref(true).unwrap();
                let pointee = self.rs_layout_of(pointee);
                assert!(pointee.is_sized());
                let size = Int::from(pointee.size.bytes());
                let offset = build::div_exact(offset_bytes, build::const_int_typed::<isize>(size));
                // If required, cast to unsized type.
                let offset = if unsigned { build::int_cast::<usize>(offset) } else { offset };

                let stmt = Statement::Assign { destination, source: offset };
                let terminator = Terminator::Goto(self.bb_name_map[&target.unwrap()]);
                return TerminatorResult { stmts: list![stmt], terminator };
            }
            rs::sym::ctpop => {
                let v = self.translate_operand(&args[0].node, span);
                let destination = self.translate_place(&destination, span);

                let val = build::count_ones(v);
                let stmt = Statement::Assign { destination, source: val };

                let terminator = Terminator::Goto(self.bb_name_map[&target.unwrap()]);
                return TerminatorResult { stmts: list![stmt], terminator };
            }
            rs::sym::exact_div => {
                let l = self.translate_operand(&args[0].node, span);
                let r = self.translate_operand(&args[1].node, span);
                let destination = self.translate_place(&destination, span);

                let val = build::div_exact(l, r);

                let stmt = Statement::Assign { destination, source: val };
                let terminator = Terminator::Goto(self.bb_name_map[&target.unwrap()]);

                return TerminatorResult { stmts: list![stmt], terminator };
            }
            rs::sym::size_of_val => {
                let destination = self.translate_place(destination, span);
                let ptr = self.translate_operand(&args[0].node, span);
                let ty = self.translate_ty(intrinsic.args.type_at(0), span);
                let stmt = Statement::Assign {
                    destination,
                    source: build::compute_size(ty, build::get_metadata(ptr)),
                };
                let terminator = Terminator::Goto(self.bb_name_map[&target.unwrap()]);
                TerminatorResult { stmts: list![stmt], terminator }
            }
            rs::sym::align_of_val => {
                let destination = self.translate_place(destination, span);
                let ptr = self.translate_operand(&args[0].node, span);
                let ty = self.translate_ty(intrinsic.args.type_at(0), span);
                let stmt = Statement::Assign {
                    destination,
                    source: build::compute_align(ty, build::get_metadata(ptr)),
                };
                let terminator = Terminator::Goto(self.bb_name_map[&target.unwrap()]);
                TerminatorResult { stmts: list![stmt], terminator }
            }
            rs::sym::unlikely | rs::sym::likely => {
                // FIXME: use the "fallback body" provided in the standard library.
                let destination = self.translate_place(&destination, span);
                let val = self.translate_operand(&args[0].node, span);

                let stmt = Statement::Assign { destination, source: val };
                let terminator = Terminator::Goto(self.bb_name_map[&target.unwrap()]);

                return TerminatorResult { stmts: list![stmt], terminator };
            }
            rs::sym::cold_path => {
                // Just a NOP for us.
                let terminator = Terminator::Goto(self.bb_name_map[&target.unwrap()]);
                return TerminatorResult { stmts: list![], terminator };
            }
            rs::sym::catch_unwind => {
                let arguments =
                    args.iter().map(|x| self.translate_operand(&x.node, x.span)).collect();
                let ret_place = self.translate_place(destination, span);
                let terminator = self.translate_catch_unwind(
                    arguments,
                    ret_place,
                    self.bb_name_map[&target.unwrap()],
                );
                TerminatorResult { stmts: list![], terminator }
            }
            rs::sym::abort => {
                let terminator = build::abort();
                TerminatorResult { stmts: list![], terminator }
            }
            name => rs::span_bug!(span, "unsupported Rust intrinsic `{}`", name),
        }
    }

    /// Translates an `UnwindAction` to MiniRust.
    /// May insert a new block into `self`, as MiniRust does not support `UnwindAction` directly.
    /// Returns the name of the first cleanup block to execute in case of unwinding.
    fn translate_unwind_action(
        &mut self,
        unwind: rs::UnwindAction,
        bb: &rs::BasicBlockData<'tcx>,
    ) -> BbName {
        // FIXME: cache and reuse the new blocks generated by this function.
        match unwind {
            rs::UnwindAction::Continue => {
                // UnwindAction::Continue must only be used by a call in a regular block,
                // therefore the unwind block should be a cleanup block.
                assert!(
                    !bb.is_cleanup,
                    "A call in a cleanup block cannot use UnwindAction::Continue"
                );
                let block_kind = BbKind::Cleanup;
                let unwind_bb_name = self.fresh_bb_name();
                let unwind_bb = BasicBlock {
                    statements: list![],
                    terminator: Terminator::ResumeUnwind,
                    kind: block_kind,
                };
                self.blocks.insert(unwind_bb_name, unwind_bb);
                unwind_bb_name
            }
            rs::UnwindAction::Unreachable => {
                let block_kind = if bb.is_cleanup { BbKind::Terminate } else { BbKind::Cleanup };
                let unwind_bb_name = self.fresh_bb_name();
                let unwind_bb = BasicBlock {
                    statements: list![],
                    terminator: Terminator::Unreachable,
                    kind: block_kind,
                };
                self.blocks.insert(unwind_bb_name, unwind_bb);
                unwind_bb_name
            }
            rs::UnwindAction::Terminate(_) => {
                // FIXME: do not ignore UnwindTerminateReason
                let block_kind = if bb.is_cleanup { BbKind::Terminate } else { BbKind::Cleanup };
                let unwind_bb_name = self.fresh_bb_name();
                let unwind_bb = BasicBlock {
                    statements: list![],
                    terminator: build::abort(),
                    kind: block_kind,
                };
                self.blocks.insert(unwind_bb_name, unwind_bb);
                unwind_bb_name
            }
            rs::UnwindAction::Cleanup(cleanup_block) => self.bb_name_map[&cleanup_block],
        }
    }

    /// Simulates a panic function.
    /// Returns a terminator that either initiates unwinding or aborts, based on the chosen panic strategy.
    fn translate_panic(
        &mut self,
        unwind: rs::UnwindAction,
        bb: &rs::BasicBlockData<'tcx>,
    ) -> Terminator {
        // FIXME: we should just call into the panic runtime here instead of replicating what it does.
        let panic_strategy = self.cx.tcx.sess.panic_strategy();
        match panic_strategy {
            rustc_target::spec::PanicStrategy::Unwind => {
                let cleanup_block = self.translate_unwind_action(unwind, bb);
                Terminator::StartUnwind {
                    // FIXME: Use actual unwind payload instead of temporary null pointer.
                    unwind_payload: build::unit_ptr(),
                    unwind_block: cleanup_block,
                }
            }
            rustc_target::spec::PanicStrategy::Abort => build::abort(),
        }
    }

    fn translate_call(
        &mut self,
        func: &rs::Operand<'tcx>,
        rs_args: &[rs::Spanned<rs::Operand<'tcx>>],
        destination: &rs::Place<'tcx>,
        target: &Option<rs::BasicBlock>,
        span: rs::Span,
        unwind: rs::UnwindAction,
        bb: &rs::BasicBlockData<'tcx>,
    ) -> TerminatorResult {
        // FIXME: func operand still needs to be evaluated in some way
        let fn_ty = func.ty(&self.body, self.tcx);
        let (f, substs_ref) = match *fn_ty.kind() {
            rs::TyKind::FnDef(id, substs) => (id, substs),
            rs::TyKind::FnPtr(signature, header) => {
                let func = self.translate_operand(func, span);

                // combine with info from the header so we can call fn_abi_of_fn_ptr
                let signature = signature.with(header);

                let abi = self.fn_abi_of_fn_ptr(signature, rs::List::empty());
                let conv = translate_calling_convention(abi.conv);

                // FIXME: deduplicate this with the argument handling below. In particular,
                // technically we also need the tuple argument handling here...
                let args: List<_> = rs_args
                    .iter()
                    .map(|x| {
                        match &x.node {
                            rs::Operand::Move(place) =>
                                ArgumentExpr::InPlace(self.translate_place(place, x.span)),
                            op => ArgumentExpr::ByValue(self.translate_operand(op, x.span)),
                        }
                    })
                    .collect();

                let unwind_block = Some(self.translate_unwind_action(unwind, bb));

                let terminator = Terminator::Call {
                    callee: func,
                    calling_convention: conv,
                    arguments: args,
                    ret: self.translate_place(&destination, span),
                    next_block: target.as_ref().map(|t| self.bb_name_map[t]),
                    unwind_block,
                };

                return TerminatorResult { terminator, stmts: List::new() };
            }
            _ => panic!(),
        };
        let instance =
            rs::Instance::expect_resolve(self.tcx, self.typing_env(), f, substs_ref, span);

        if matches!(instance.def, rs::InstanceKind::Intrinsic(_)) {
            // A Rust intrinsic.
            return self.translate_rs_intrinsic(instance, rs_args, destination, target, span);
        }

        let terminator = if self.tcx.crate_name(f.krate).as_str() == "intrinsics" {
            // Direct call to a MiniRust intrinsic.
            let intrinsic = match self.tcx.item_name(f).as_str() {
                "print" => IntrinsicOp::PrintStdout,
                "eprint" => IntrinsicOp::PrintStderr,
                "exit" => IntrinsicOp::Exit,
                "panic" => IntrinsicOp::Abort,
                "allocate" => IntrinsicOp::Allocate,
                "deallocate" => IntrinsicOp::Deallocate,
                "spawn" => IntrinsicOp::Spawn,
                "join" => IntrinsicOp::Join,
                "create_lock" => IntrinsicOp::Lock(IntrinsicLockOp::Create),
                "acquire" => IntrinsicOp::Lock(IntrinsicLockOp::Acquire),
                "release" => IntrinsicOp::Lock(IntrinsicLockOp::Release),
                "atomic_store" => IntrinsicOp::AtomicStore,
                "atomic_load" => IntrinsicOp::AtomicLoad,
                "compare_exchange" => IntrinsicOp::AtomicCompareExchange,
                "atomic_fetch_add" => IntrinsicOp::AtomicFetchAndOp(IntBinOp::Add),
                "atomic_fetch_sub" => IntrinsicOp::AtomicFetchAndOp(IntBinOp::Sub),
                name => panic!("unsupported MiniRust intrinsic `{}`", name),
            };
            Terminator::Intrinsic {
                intrinsic,
                arguments: rs_args
                    .iter()
                    .map(|x| self.translate_operand(&x.node, x.span))
                    .collect(),
                ret: self.translate_place(&destination, span),
                next_block: target.as_ref().map(|t| self.bb_name_map[t]),
            }
        } else if is_panic_fn(&instance.to_string()) {
            // We can't translate this call, it takes a string. As a hack we just ignore the argument.
            self.translate_panic(unwind, bb)
        } else {
            let abi = self
                .cx
                .tcx
                .fn_abi_of_instance(self.typing_env().as_query_input((instance, rs::List::empty())))
                .unwrap();
            let conv = translate_calling_convention(abi.conv);

            let fn_sig = fn_ty.fn_sig(self.tcx);
            let mut args: List<ArgumentExpr> = if fn_sig.abi() == rustc_abi::ExternAbi::RustCall
                && !rs_args.is_empty()
            {
                // Untuple the last argument
                let (tuple_arg, other_args) = rs_args.split_last().unwrap();

                match &tuple_arg.node {
                    rs::Operand::Move(tuple_place) | rs::Operand::Copy(tuple_place) => {
                        let tuple_ty = tuple_arg.node.ty(&self.body, self.tcx);

                        let rs::TyKind::Tuple(tuple_tys) = tuple_ty.kind() else {
                            panic!("Expected tuple for rust-call last argument");
                        };

                        let tuple_args: Vec<_> = tuple_tys
                            .iter()
                            .enumerate()
                            .map(|(i, ty)| {
                                let field_place = tuple_place.project_deeper(
                                    &[rs::PlaceElem::Field(rs::FieldIdx::from_usize(i), ty)],
                                    self.tcx,
                                );
                                ArgumentExpr::InPlace(
                                    self.translate_place(&field_place, tuple_arg.span),
                                )
                            })
                            .collect();

                        other_args
                            .iter()
                            .map(|x| {
                                match &x.node {
                                    rs::Operand::Move(place) =>
                                        ArgumentExpr::InPlace(self.translate_place(place, x.span)),
                                    op => ArgumentExpr::ByValue(self.translate_operand(op, x.span)),
                                }
                            })
                            .chain(tuple_args)
                            .collect()
                    }
                    _ => panic!("Expected Move or Copy operand for rust-call tuple argument"),
                }
            } else {
                rs_args
                    .iter()
                    .map(|x| {
                        match &x.node {
                            rs::Operand::Move(place) =>
                                ArgumentExpr::InPlace(self.translate_place(place, x.span)),
                            op => ArgumentExpr::ByValue(self.translate_operand(op, x.span)),
                        }
                    })
                    .collect()
            };

            let unwind_block = Some(self.translate_unwind_action(unwind, bb));

            // Distinguish direct function calls or dynamic dispatch on a trait object.
            let callee = if let rs::InstanceKind::Virtual(_trait, method) = instance.def {
                // FIXME: This does not implement all receivers as allowed by `std::ops::DispatchFromDyn`.
                // properly implementing this requires finding the right field to extract the
                // pointer value, and coming up with a suitable type for passing the receiver
                // to the callee. We can't know the exact type so some approximation will
                // have to suffice.
                // See <https://github.com/minirust/minirust/issues/257>.
                let receiver = self.translate_operand(&rs_args[0].node, rs_args[0].span);
                let adjusted_receiver = build::by_value(build::get_thin_pointer(receiver));
                args.set(Int::from(0), adjusted_receiver);

                // We built the vtables to have the method indices as method names.
                let method = TraitMethodName(Name::from_internal(method as u32));

                build::vtable_method_lookup(build::get_metadata(receiver), method)
            } else {
                build::fn_ptr(self.cx.get_fn_name(instance))
            };

            Terminator::Call {
                callee,
                calling_convention: conv,
                arguments: args,
                ret: self.translate_place(&destination, span),
                next_block: target.as_ref().map(|t| self.bb_name_map[t]),
                unwind_block,
            }
        };
        TerminatorResult { terminator, stmts: List::new() }
    }
}

// HACK to skip translating some functions we can't handle yet.
// These always panic so we use `translate_panic` instead.
fn is_panic_fn(name: &str) -> bool {
    let fns = [
        "core::panicking::panic",
        "core::panicking::panic_fmt",
        "core::panicking::panic_nounwind",
        "core::panicking::panic_nounwind_fmt",
        "core::slice::index::slice_start_index_len_fail",
        "core::slice::index::slice_end_index_len_fail",
        "core::slice::index::slice_end_index_overflow_fail",
        "core::slice::index::slice_index_order_fail",
        "core::str::slice_error_fail",
    ];
    fns.contains(&name)
}
