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
            assert!(discr.ty(&fcx.body, fcx.cx.tcx).is_bool()); // for now we only support bool branching.

            let condition = translate_operand(discr, fcx);
            let then_block = targets.target_for_value(1);
            let then_block = fcx.bb_name_map[&then_block];

            let else_block = targets.target_for_value(0);
            let else_block = fcx.bb_name_map[&else_block];

            Terminator::If {
                condition,
                then_block,
                else_block,
            }
        }
        // those are IGNORED currently.
        rs::TerminatorKind::Drop { target, .. } | rs::TerminatorKind::Assert { target, .. } => {
            Terminator::Goto(fcx.bb_name_map[&target])
        }
        x => {
            dbg!(x);
            todo!()
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
    let key = (f, substs_ref);
    let instance = rs::Instance::resolve(
        fcx.cx.tcx,
        rs::ParamEnv::empty(),
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
            name => panic!("unsupported intrinsic `{}`", name),
        };
        Terminator::CallIntrinsic {
            intrinsic,
            arguments: args.iter().map(|x| translate_operand(x, fcx)).collect(),
            ret: translate_place(&destination, fcx),
            next_block: target.as_ref().map(|t| fcx.bb_name_map[t]),
        }
    } else {
        let abi = fcx.cx.tcx.fn_abi_of_instance(rs::ParamEnv::empty().and((instance, rs::List::empty()))).unwrap();
        let conv = translate_calling_convention(abi.conv);

        let args: List<_> = args.iter().map(|op| match op {
            rs::Operand::Move(place) => ArgumentExpr::InPlace(translate_place(place, fcx)),
            op => {
                let ty = op.ty(&fcx.body, fcx.cx.tcx);
                let align = layout_of(ty, fcx.cx.tcx).align;
                ArgumentExpr::ByValue(translate_operand(op, fcx), align)
            },
        }).collect();

        if !fcx.cx.fn_name_map.contains_key(&key) {
            let fn_name = fcx.cx.fn_name_map.len();
            let fn_name = FnName(Name::from_internal(fn_name as _));
            fcx.cx.fn_name_map.insert(key, fn_name);
        }
        Terminator::Call {
            callee: build::fn_ptr_conv(fcx.cx.fn_name_map[&key].0.get_internal(), conv),
            arguments: args,
            ret: translate_place(&destination, fcx),
            next_block: target.as_ref().map(|t| fcx.bb_name_map[t]),
        }
    }
}
