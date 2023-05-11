use crate::*;

#[test]
fn alloc_success() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Allocate,
            arguments: list![const_int::<usize>(4), const_int::<usize>(4)],
            ret: Some(local(0)),
            next_block: Some(BbName(Name::from_internal(1))),
        },
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f], &[]);
    dump_program(p);
    assert_stop(p);
}

#[test]
fn alloc_argcount() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Allocate,
            arguments: list![],
            ret: Some(local(0)),
            next_block: None,
        },
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f], &[]);
    dump_program(p);
    assert_ub(p, "invalid number of arguments for `Intrinsic::Allocate`");
}

#[test]
fn alloc_align_err() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Allocate,
            arguments: list![const_int::<usize>(4), const_int::<usize>(13)], // 13 is no power of two! hence error!
            ret: Some(local(0)),
            next_block: Some(BbName(Name::from_internal(1))),
        },
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f], &[]);
    dump_program(p);
    assert_ub(p, "invalid alignment for `Intrinsic::Allocate`: not a power of 2");
}

#[test]
fn alloc_size_err() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Allocate,
            arguments: list![const_int::<isize>(-1), const_int::<usize>(4)], // -1 is not a valid size!
            ret: Some(local(0)),
            next_block: Some(BbName(Name::from_internal(1))),
        },
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f], &[]);
    dump_program(p);
    assert_ub(p, "invalid size for `Intrinsic::Allocate`: negative size");
}

#[test]
fn alloc_wrongarg1() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Allocate,
            // First argument should be an int, so bool is unexpected here!
            arguments: list![const_bool(true), const_int::<usize>(4)],
            ret: Some(local(0)),
            next_block: Some(BbName(Name::from_internal(1))),
        },
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f], &[]);
    dump_program(p);
    assert_ub(p, "invalid first argument to `Intrinsic::Allocate`");
}

#[test]
fn alloc_wrongarg2() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Allocate,
            // Second argument should be an int, so bool is unexpected here!
            arguments: list![const_int::<usize>(4), const_bool(true)],
            ret: Some(local(0)),
            next_block: Some(BbName(Name::from_internal(1))),
        },
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f], &[]);
    dump_program(p);
    assert_ub(p, "invalid second argument to `Intrinsic::Allocate`");
}

#[test]
fn alloc_wrongreturn() {
    let locals = [ <()>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Allocate,
            arguments: list![const_int::<usize>(4), const_int::<usize>(4)],
            ret: Some(local(0)),
            next_block: Some(BbName(Name::from_internal(1))),
        },
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f], &[]);
    dump_program(p);
    assert_ub(p, "invalid return type for `Intrinsic::Allocate`");
}
