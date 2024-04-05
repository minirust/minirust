use crate::*;

// Some Rust features are not supported, and are ignored by `minimize`.
// Those can be found by grepping "IGNORED".

/// A MIR statement becomes either a MiniRust statement or an intrinsic with some arguments, which
/// then starts a new basic block.
enum StatementResult {
    Statement(Statement),
    Intrinsic { intrinsic: IntrinsicOp, destination: PlaceExpr, arguments: List<ValueExpr> },
}

impl<'cx, 'tcx> FnCtxt<'cx, 'tcx> {
    /// Translate the given basic block and insert it into `self` with the given name.
    /// May insert more than one block because some MIR statements turn into MiniRust terminators.
    pub fn translate_bb(&mut self, name: BbName, bb: &rs::BasicBlockData<'tcx>) {
        let mut cur_block_name = name;
        let mut cur_block_statements = List::new();
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
                    let cur_block = BasicBlock { statements: cur_block_statements, terminator };
                    let old = self.blocks.insert(cur_block_name, cur_block);
                    assert!(old.is_none()); // make sure we do not overwrite a bb
                    // Go on building the next block.
                    cur_block_name = next_bb;
                    cur_block_statements = List::new();
                }
            }
        }
        let terminator = self.translate_terminator(bb.terminator());
        let cur_block = BasicBlock { statements: cur_block_statements, terminator };
        let old = self.blocks.insert(cur_block_name, cur_block);
        assert!(old.is_none()); // make sure we do not overwrite a bb
    }

    fn translate_stmt(&mut self, stmt: &rs::Statement<'tcx>) -> StatementResult {
        StatementResult::Statement(match &stmt.kind {
            rs::StatementKind::Assign(box (place, rval)) => {
                let destination = self.translate_place(place);
                // Some things that MIR handles as rvalues are non-deterministic,
                // so MiniRust treats them differently.
                match rval {
                    rs::Rvalue::Cast(rs::CastKind::PointerExposeAddress, operand, _) => {
                        let operand = self.translate_operand(operand);
                        return StatementResult::Intrinsic {
                            intrinsic: IntrinsicOp::PointerExposeProvenance,
                            destination,
                            arguments: list![operand],
                        };
                    }
                    rs::Rvalue::Cast(rs::CastKind::PointerFromExposedAddress, operand, _) => {
                        // TODO untested so far! (Can't test because of `predict`)
                        let operand = self.translate_operand(operand);
                        return StatementResult::Intrinsic {
                            intrinsic: IntrinsicOp::PointerWithExposedProvenance,
                            destination,
                            arguments: list![operand],
                        };
                    }
                    _ => {}
                }
                let source = self.translate_rvalue(rval);
                Statement::Assign { destination, source }
            }
            rs::StatementKind::StorageLive(local) =>
                Statement::StorageLive(self.local_name_map[&local]),
            rs::StatementKind::StorageDead(local) =>
                Statement::StorageDead(self.local_name_map[&local]),
            rs::StatementKind::Retag(kind, place) => {
                let place = self.translate_place(place);
                let fn_entry = matches!(kind, rs::RetagKind::FnEntry);
                Statement::Validate { place, fn_entry }
            }
            rs::StatementKind::Deinit(place) => {
                let place = self.translate_place(place);
                Statement::Deinit { place }
            }
            rs::StatementKind::SetDiscriminant { place, variant_index } => {
                let place_ty =
                    rs::Place::ty_from(place.local, place.projection, &self.body, self.tcx).ty;
                let discriminant = self.discriminant_for_variant(place_ty, *variant_index);
                Statement::SetDiscriminant {
                    destination: self.translate_place(place),
                    value: discriminant,
                }
            }
            rs::StatementKind::Intrinsic(box rs::NonDivergingIntrinsic::Assume(op)) => {
                let op = self.translate_operand(op);
                // Doesn't return anything, get us a dummy place.
                let destination = build::zst_place();
                return StatementResult::Intrinsic {
                    intrinsic: IntrinsicOp::Assume,
                    destination,
                    arguments: list![op],
                };
            }
            x => {
                dbg!(x);
                todo!()
            }
        })
    }

    fn translate_terminator(&mut self, terminator: &rs::Terminator<'tcx>) -> Terminator {
        match &terminator.kind {
            rs::TerminatorKind::Return => Terminator::Return,
            rs::TerminatorKind::Goto { target } => Terminator::Goto(self.bb_name_map[&target]),
            rs::TerminatorKind::Call { func, target, destination, args, .. } =>
                self.translate_call(func, args, destination, target),
            rs::TerminatorKind::SwitchInt { discr, targets } => {
                let ty = self.translate_ty(discr.ty(&self.body, self.tcx));

                let discr_op = self.translate_operand(discr);
                let (value, int_ty) = match ty {
                    Type::Bool => {
                        // If the value is a boolean we need to cast it to an integer first as MiniRust switch only operates on ints.
                        let Type::Int(u8_inttype) = <u8>::get_type() else { unreachable!() };
                        (
                            ValueExpr::UnOp {
                                operator: UnOp::Cast(CastOp::BoolToInt(u8_inttype)),
                                operand: GcCow::new(discr_op),
                            },
                            u8_inttype,
                        )
                    }
                    Type::Int(ity) => (discr_op, ity),
                    // FIXME: add support for switching on `char`
                    _ => panic!("SwitchInt terminator currently only supports int and bool."),
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
            // those are IGNORED currently.
            rs::TerminatorKind::Drop { target, .. } | rs::TerminatorKind::Assert { target, .. } =>
                Terminator::Goto(self.bb_name_map[&target]),
            x => {
                unimplemented!("terminator not supported: {x:?}")
            }
        }
    }

    fn translate_call(
        &mut self,
        func: &rs::Operand<'tcx>,
        args: &[rs::Spanned<rs::Operand<'tcx>>],
        destination: &rs::Place<'tcx>,
        target: &Option<rs::BasicBlock>,
    ) -> Terminator {
        // For now we only support calling specific functions, not function pointers.
        let rs::Operand::Constant(box f1) = func else { panic!() };
        let rs::mir::Const::Val(_, f2) = f1.const_ else { panic!() };
        let &rs::TyKind::FnDef(f, substs_ref) = f2.kind() else { panic!() };
        let instance =
            rs::Instance::expect_resolve(self.tcx, rs::ParamEnv::reveal_all(), f, substs_ref);

        if self.tcx.crate_name(f.krate).as_str() == "intrinsics" {
            let intrinsic = match self.tcx.item_name(f).as_str() {
                "print" => IntrinsicOp::PrintStdout,
                "eprint" => IntrinsicOp::PrintStderr,
                "exit" => IntrinsicOp::Exit,
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
                "atomic_fetch_add" => IntrinsicOp::AtomicFetchAndOp(BinOpInt::Add),
                "atomic_fetch_sub" => IntrinsicOp::AtomicFetchAndOp(BinOpInt::Sub),
                name => panic!("unsupported intrinsic `{}`", name),
            };
            Terminator::Intrinsic {
                intrinsic,
                arguments: args.iter().map(|x| self.translate_operand(&x.node)).collect(),
                ret: self.translate_place(&destination),
                next_block: target.as_ref().map(|t| self.bb_name_map[t]),
            }
        } else if is_panic_fn(&instance.to_string()) {
            // We can't translate this call, it takes a string. So as a special hack we just make this `Unreachable`.
            Terminator::Unreachable
        } else {
            let abi = self
                .cx
                .tcx
                .fn_abi_of_instance(rs::ParamEnv::reveal_all().and((instance, rs::List::empty())))
                .unwrap();
            let conv = translate_calling_convention(abi.conv);

            let args: List<_> = args
                .iter()
                .map(|op| {
                    match &op.node {
                        rs::Operand::Move(place) =>
                            ArgumentExpr::InPlace(self.translate_place(place)),
                        op => ArgumentExpr::ByValue(self.translate_operand(op)),
                    }
                })
                .collect();

            Terminator::Call {
                callee: build::fn_ptr_conv(self.cx.get_fn_name(instance).0.get_internal(), conv),
                arguments: args,
                ret: self.translate_place(&destination),
                next_block: target.as_ref().map(|t| self.bb_name_map[t]),
            }
        }
    }
}

// HACK to skip translating some functions we can't handle yet.
fn is_panic_fn(name: &str) -> bool {
    name == "core::panicking::panic" || name == "core::panicking::panic_nounwind"
}
