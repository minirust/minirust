use crate::*;

#[test]
fn neg_count_array() {
    let ty = array_ty(<()>::get_type(), -1);
    let locals = &[ty];

    let stmts = &[storage_live(0)];

    let p = small_program(locals, stmts);
    dump_program(p);
    assert_ill_formed::<BasicMem>(p, "Type::Array: negative amount of elements");
}

#[test]
fn no_main() {
    let p = program(&[]);
    assert_ill_formed::<BasicMem>(p, "Program: start function does not exist");
}

#[test]
fn too_large_local() {
    let ty = <[u8; usize::MAX / 2 + 1]>::get_type();

    let locals = &[ty];
    let stmts = &[];

    let prog = small_program(locals, stmts);
    assert_ill_formed::<BasicMem>(prog, "LayoutStrategy: size not valid");
}

#[test]
fn type_mismatch() {
    let locals = &[<i32>::get_type()];
    let stmts = &[storage_live(0), assign(local(0), const_int::<u32>(0))];
    let p = small_program(locals, stmts);
    assert_ill_formed::<BasicMem>(p, "Statement::Assign: destination and source type differ");
}

#[test]
fn goto_wrong_blockkind() {
    let bb0 = block!(goto(1));
    let bb1 = block(&[], exit(), BbKind::Cleanup);
    let f = function(Ret::No, 0, &[], &[bb0, bb1]);
    let p = program(&[f]);
    dump_program(p);
    assert_ill_formed::<BasicMem>(p, "Terminator::Goto: next block has the wrong block kind");
}

#[test]
fn switch_wrong_blockkind() {
    let bb0 = block!(switch_int(const_int(0), &[(0u8, 1), (1u8, 1), (7u8, 2)], 1));
    let bb1 = block!(exit());
    let bb2 = block(&[], exit(), BbKind::Cleanup);
    let f = function(Ret::No, 0, &[], &[bb0, bb1, bb2]);
    let p = program(&[f]);
    dump_program(p);
    assert_ill_formed::<BasicMem>(
        p,
        "Terminator::Switch: next block in case 7 has the wrong block kind",
    );
}

#[test]
fn switch_wrong_blockkind_fallback() {
    let bb0 = block!(switch_int(const_int(0), &[(0u8, 1), (1u8, 1), (7u8, 1)], 2));
    let bb1 = block!(exit());
    let bb2 = block(&[], exit(), BbKind::Cleanup);
    let f = function(Ret::No, 0, &[], &[bb0, bb1, bb2]);
    let p = program(&[f]);
    dump_program(p);
    assert_ill_formed::<BasicMem>(p, "Terminator::Switch: fallback block has the wrong block kind");
}

#[test]
fn intrinsic_wrong_blockkind() {
    let bb0 = block!(print(const_int(0), 1));
    let bb1 = block(&[], exit(), BbKind::Cleanup);
    let f = function(Ret::No, 0, &[], &[bb0, bb1]);
    let p = program(&[f]);
    dump_program(p);
    assert_ill_formed::<BasicMem>(p, "Terminator::Intrinsic: next block has the wrong block kind");
}

#[test]
fn call_nextblock_wrong_kind() {
    let bb0 = block!(Terminator::Call {
        callee: fn_ptr_internal(1),
        calling_convention: CallingConvention::C,
        arguments: list![],
        ret: unit_place(),
        next_block: Some(BbName(Name::from_internal(1))),
        unwind_block: None,
    });
    let bb1 = block(&[], exit(), BbKind::Terminate);
    let f0 = function(Ret::No, 0, &[], &[bb0, bb1]);

    let f1 = function(Ret::No, 0, &[], &[block!(return_())]);
    let p = program(&[f0, f1]);
    dump_program(p);
    assert_ill_formed::<BasicMem>(p, "Terminator::Call: next block has the wrong block kind");
}

#[test]
fn call_unwindblock_wrong_kind() {
    let bb0 = block!(Terminator::Call {
        callee: fn_ptr_internal(1),
        calling_convention: CallingConvention::C,
        arguments: list![],
        ret: unit_place(),
        next_block: None,
        unwind_block: Some(BbName(Name::from_internal(1))),
    });
    let bb1 = block!(exit());
    let f0 = function(Ret::No, 0, &[], &[bb0, bb1]);

    let f1 = function(Ret::No, 0, &[], &[block!(return_())]);
    let p = program(&[f0, f1]);
    dump_program(p);
    assert_ill_formed::<BasicMem>(p, "Terminator::Call: unwind block has the wrong block kind");
}

#[test]
fn call_unwindblock_wrong_kind_2() {
    let bb0 = block!(start_unwind(BbName(Name::from_internal(1))));
    let bb1 = block(
        &[],
        Terminator::Call {
            callee: fn_ptr_internal(1),
            calling_convention: CallingConvention::C,
            arguments: list![],
            ret: unit_place(),
            next_block: None,
            unwind_block: Some(BbName(Name::from_internal(2))),
        },
        BbKind::Cleanup,
    );
    let bb2 = block(&[], exit(), BbKind::Cleanup);
    let f0 = function(Ret::No, 0, &[], &[bb0, bb1, bb2]);

    let f1 = function(Ret::No, 0, &[], &[block!(return_())]);
    let p = program(&[f0, f1]);
    dump_program(p);
    assert_ill_formed::<BasicMem>(p, "Terminator::Call: unwind block has the wrong block kind");
}

#[test]
fn return_in_cleanup() {
    let mut p = ProgramBuilder::new();
    let f = {
        let mut f = p.declare_function();
        let c = f.cleanup(|f| f.return_());
        f.start_unwind(c);
        p.finish_function(f)
    };

    let main_fn = {
        let mut main_fn = p.declare_function();
        let c = main_fn.cleanup(|f| {
            f.abort();
        });
        main_fn.call(unit_place(), fn_ptr(f), &[], c);
        main_fn.exit();
        p.finish_function(main_fn)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_ill_formed::<BasicMem>(p, "Terminator::Return has to be called in a regular block");
}

#[test]
fn start_unwind_in_cleanup() {
    let mut p = ProgramBuilder::new();
    let f = {
        let mut f = p.declare_function();
        let outer_cleanup = f.cleanup(|f| {
            let inner_cleanup = f.cleanup(|f| f.exit());
            f.start_unwind(inner_cleanup);
        });
        f.start_unwind(outer_cleanup);
        p.finish_function(f)
    };
    let p = p.finish_program(f);
    dump_program(p);
    assert_ill_formed::<BasicMem>(p, "Terminator::StartUnwind has to be called in regular block");
}

#[test]
fn start_unwind_wrong_nextblock() {
    let bb0 = block!(start_unwind(BbName(Name::from_internal(1))));
    let bb1 = block!(exit());
    let f = function(Ret::No, 0, &[], &[bb0, bb1]);
    let p = program(&[f]);
    dump_program(p);
    assert_ill_formed::<BasicMem>(
        p,
        "Terminator::StartUnwind: next block has the wrong block kind",
    );
}

#[test]
fn resume_in_regular_block() {
    let mut p = ProgramBuilder::new();
    let f = {
        let mut f = p.declare_function();
        f.resume_unwind();
        p.finish_function(f)
    };
    let p = p.finish_program(f);
    dump_program(p);
    assert_ill_formed::<BasicMem>(p, "Terminator::ResumeUnwind: has to be called in cleanup block");
}
