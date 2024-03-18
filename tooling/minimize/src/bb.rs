use crate::*;

// Some Rust features are not supported, and are ignored by `minimize`.
// Those can be found by grepping "IGNORED".

impl<'cx, 'tcx> FnCtxt<'cx, 'tcx> {
    pub fn translate_bb(&mut self, bb: &rs::BasicBlockData<'tcx>) -> BasicBlock {
        let mut statements = List::new();
        for stmt in bb.statements.iter() {
            // unsupported statements will be IGNORED.
            if let Some(stmts) = self.translate_stmt(stmt) {
                for stmt in stmts {
                    statements.push(stmt);
                }
            }
        }
        BasicBlock { statements, terminator: self.translate_terminator(bb.terminator()) }
    }

    fn translate_stmt(&mut self, stmt: &rs::Statement<'tcx>) -> Option<Vec<Statement>> {
        Some(match &stmt.kind {
            rs::StatementKind::Assign(box (place, rval)) => {
                let destination = self.translate_place(place);
                let (mut stmts, source) = self.translate_rvalue(rval)?; // assign of unsupported rvalues are IGNORED.

                // this puts the extra statements before the evaluation of `destination`!
                stmts.push(Statement::Assign { destination, source });
                stmts
            }
            rs::StatementKind::StorageLive(local) => {
                vec![Statement::StorageLive(self.local_name_map[&local])]
            }
            rs::StatementKind::StorageDead(local) => {
                vec![Statement::StorageDead(self.local_name_map[&local])]
            }
            rs::StatementKind::Retag(kind, place) => {
                let place = self.translate_place(place);
                let fn_entry = matches!(kind, rs::RetagKind::FnEntry);
                vec![Statement::Validate { place, fn_entry }]
            }
            rs::StatementKind::Deinit(place) => {
                let place = self.translate_place(place);
                vec![Statement::Deinit { place }]
            }
            rs::StatementKind::SetDiscriminant { place, variant_index } => {
                let place_ty =
                    rs::Place::ty_from(place.local, place.projection, &self.body, self.tcx).ty;
                let discriminant = self.discriminant_for_variant(place_ty, *variant_index);
                vec![Statement::SetDiscriminant {
                    destination: self.translate_place(place),
                    value: discriminant,
                }]
            }
            // FIXME: add assume intrinsic statement to MiniRust.
            rs::StatementKind::Intrinsic(box rs::NonDivergingIntrinsic::Assume(_)) => vec![],
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
                                operator: UnOp::Bool(UnOpBool::IntCast(u8_inttype)),
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
            rs::Instance::resolve(self.tcx, rs::ParamEnv::reveal_all(), f, substs_ref)
                .unwrap()
                .unwrap();

        if self.tcx.crate_name(f.krate).as_str() == "intrinsics" {
            let intrinsic = match self.tcx.item_name(f).as_str() {
                "print" => Intrinsic::PrintStdout,
                "eprint" => Intrinsic::PrintStderr,
                "exit" => Intrinsic::Exit,
                "allocate" => Intrinsic::Allocate,
                "deallocate" => Intrinsic::Deallocate,
                "spawn" => Intrinsic::Spawn,
                "join" => Intrinsic::Join,
                "create_lock" => Intrinsic::Lock(LockIntrinsic::Create),
                "acquire" => Intrinsic::Lock(LockIntrinsic::Acquire),
                "release" => Intrinsic::Lock(LockIntrinsic::Release),
                "atomic_store" => Intrinsic::AtomicStore,
                "atomic_load" => Intrinsic::AtomicLoad,
                "compare_exchange" => Intrinsic::AtomicCompareExchange,
                "atomic_fetch_add" => Intrinsic::AtomicFetchAndOp(BinOpInt::Add),
                "atomic_fetch_sub" => Intrinsic::AtomicFetchAndOp(BinOpInt::Sub),
                name => panic!("unsupported intrinsic `{}`", name),
            };
            Terminator::CallIntrinsic {
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
