use crate::*;

// Some Rust features are not supported, and are ignored by `minimize`.
// Those can be found by grepping "IGNORED".

pub fn translate_bb<'cx, 'tcx>(
    bb: &rs::BasicBlockData<'tcx>,
    fcx: &mut FnCtxt<'cx, 'tcx>,
) -> BasicBlock {
    let mut statements = List::new();
    for stmt in bb.statements.iter() {
        // unsupported statements will be IGNORED.
        if let Some(stmts) = translate_stmt(stmt, fcx) {
            for stmt in stmts {
                statements.push(stmt);
            }
        }
    }
    BasicBlock {
        statements,
        terminator: translate_terminator(bb.terminator(), fcx),
    }
}

fn translate_stmt<'cx, 'tcx>(
    stmt: &rs::Statement<'tcx>,
    fcx: &mut FnCtxt<'cx, 'tcx>,
) -> Option<Vec<Statement>> {
    Some(match &stmt.kind {
        rs::StatementKind::Assign(box (place, rval)) => {
            let destination = translate_place(place, fcx);
            let (mut stmts, source) = translate_rvalue(rval, fcx)?; // assign of unsupported rvalues are IGNORED.
            // this puts the extra statements before the evaluation of `destination`!
            stmts.push(Statement::Assign {
                destination,
                source, 
            });
            stmts
        }
        rs::StatementKind::StorageLive(local) => vec![Statement::StorageLive(fcx.local_name_map[&local])],
        rs::StatementKind::StorageDead(local) => vec![Statement::StorageDead(fcx.local_name_map[&local])],
        rs::StatementKind::Retag(kind, place) => {
            let place = translate_place(place, fcx);
            let fn_entry = matches!(kind, rs::RetagKind::FnEntry);
            vec![Statement::Validate { place, fn_entry }]
        }
        rs::StatementKind::Deinit(place) => {
            let place = translate_place(place, fcx);
            vec![Statement::Deinit { place }]
        }
        rs::StatementKind::SetDiscriminant { place, variant_index } => {
            let place_ty = rs::Place::ty_from(
                place.local,
                place.projection,
                &fcx.body,
                fcx.cx.tcx,
            ).ty;
            let discriminant = fcx.discriminant_for_variant(place_ty, *variant_index);
            vec![Statement::SetDiscriminant { destination: translate_place(place, fcx), value: discriminant }]
        }
        // FIXME: add assume intrinsic statement to MiniRust.
        rs::StatementKind::Intrinsic(box rs::NonDivergingIntrinsic::Assume(_)) => vec![],
        x => {
            dbg!(x);
            todo!()
        }
    })
}

fn translate_terminator<'cx, 'tcx>(
    terminator: &rs::Terminator<'tcx>,
    fcx: &mut FnCtxt<'cx, 'tcx>,
) -> Terminator {
    match &terminator.kind {
        rs::TerminatorKind::Return => Terminator::Return,
        rs::TerminatorKind::Goto { target } => Terminator::Goto(fcx.bb_name_map[&target]),
        rs::TerminatorKind::Call {
            func,
            target,
            destination,
            args,
            ..
        } => translate_call(fcx, func, args, destination, target),
        rs::TerminatorKind::SwitchInt { discr, targets } => {
            let ty = fcx.translate_ty(discr.ty(&fcx.body, fcx.cx.tcx));

            let discr_op = translate_operand(discr, fcx);
            let (value, int_ty) = match ty {
                Type::Bool => {
                    // If the value is a boolean we need to cast it to an integer first as MiniRust switch only operates on ints.
                    let Type::Int(u8_inttype) = <u8>::get_type() else { unreachable!() };
                    (ValueExpr::UnOp {
                        operator: UnOp::BoolToIntCast(u8_inttype),
                        operand: GcCow::new(discr_op),
                    }, u8_inttype)
                },
                Type::Int(ity) => (discr_op, ity),
                // FIXME: add support for switching on `char`
                _ => panic!("SwitchInt terminator currently only supports int and bool.")
            };

            let cases = targets.iter().map(|(value, target)| (int_from_bits(value, int_ty), fcx.bb_name_map[&target])).collect();

            let fallback_block = targets.otherwise();
            let fallback = fcx.bb_name_map[&fallback_block];

            Terminator::Switch {
                value,
                cases,
                fallback,
            }
        }
        rs::TerminatorKind::Unreachable => Terminator::Unreachable,
        // those are IGNORED currently.
        rs::TerminatorKind::Drop { target, .. } | rs::TerminatorKind::Assert { target, .. } => {
            Terminator::Goto(fcx.bb_name_map[&target])
        }
        x => {
            unimplemented!("terminator not supported: {x:?}")
        }
    }
}

fn translate_call<'cx, 'tcx>(
    fcx: &mut FnCtxt<'cx, 'tcx>,
    func: &rs::Operand<'tcx>,
    args: &[rs::Operand<'tcx>],
    destination: &rs::Place<'tcx>,
    target: &Option<rs::BasicBlock>,
) -> Terminator {
    // For now we only support calling specific functions, not function pointers.
    let rs::Operand::Constant(box f1) = func else { panic!() };
    let rs::ConstantKind::Val(_, f2) = f1.literal else { panic!() };
    let &rs::TyKind::FnDef(f, substs_ref) = f2.kind() else { panic!() };
    let instance = rs::Instance::resolve(
        fcx.cx.tcx,
        rs::ParamEnv::reveal_all(),
        f,
        substs_ref,
    ).unwrap().unwrap();

    if fcx.cx.tcx.crate_name(f.krate).as_str() == "intrinsics" {
        let intrinsic = match fcx.cx.tcx.item_name(f).as_str() {
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
            arguments: args.iter().map(|x| translate_operand(x, fcx)).collect(),
            ret: translate_place(&destination, fcx),
            next_block: target.as_ref().map(|t| fcx.bb_name_map[t]),
        }
    } else {
        let abi = fcx.cx.tcx.fn_abi_of_instance(rs::ParamEnv::reveal_all().and((instance, rs::List::empty()))).unwrap();
        let conv = translate_calling_convention(abi.conv);

        let args: List<_> = args.iter().map(|op| match op {
            rs::Operand::Move(place) => ArgumentExpr::InPlace(translate_place(place, fcx)),
            op => ArgumentExpr::ByValue(translate_operand(op, fcx)),
        }).collect();

        
        Terminator::Call {
            callee: build::fn_ptr_conv(
                fcx.cx.get_fn_name(instance).0.get_internal(),
                conv
            ),
            arguments: args,
            ret: translate_place(&destination, fcx),
            next_block: target.as_ref().map(|t| fcx.bb_name_map[t]),
        }
    }
}
