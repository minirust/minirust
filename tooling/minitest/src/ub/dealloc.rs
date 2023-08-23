use crate::*;

#[test]
fn dealloc_success() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        allocate(const_int::<usize>(4), const_int::<usize>(4), local(0), 1)
    );
    let b1 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Deallocate,
            arguments: list![load(local(0)), const_int::<usize>(4), const_int::<usize>(4)],
            ret: None,
            next_block: Some(BbName(Name::from_internal(2))),
        },
    );
    let b2 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);
    let p = program(&[f]);
    dump_program(p);
    assert_stop(p);
}

#[test]
fn dealloc_argcount() {
    let locals = [];

    let b0 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Deallocate,
            arguments: list![],
            ret: None,
            next_block: Some(BbName(Name::from_internal(1))),
        },
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "invalid number of arguments for `Intrinsic::Deallocate`");
}

#[test]
fn dealloc_align_err() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        allocate(const_int::<usize>(4), const_int::<usize>(4), local(0), 1)
    );
    let b1 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Deallocate,
            arguments: list![load(local(0)), const_int::<usize>(4), const_int::<usize>(13)], // 13 is not a power of two!
            ret: None,
            next_block: Some(BbName(Name::from_internal(2))),
        },
    );
    let b2 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "invalid alignment for `Intrinsic::Deallocate`: not a power of 2");
}

#[test]
fn dealloc_size_err() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        allocate(const_int::<usize>(4), const_int::<usize>(4), local(0), 1)
    );
    let b1 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Deallocate,
            arguments: list![load(local(0)), const_int::<isize>(-1), const_int::<usize>(4)], // -1 is not a valid size!
            ret: None,
            next_block: Some(BbName(Name::from_internal(2))),
        },
    );
    let b2 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "invalid size for `Intrinsic::Deallocate`: negative size");
}

#[test]
fn dealloc_wrongarg1() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        allocate(const_int::<usize>(4), const_int::<usize>(4), local(0), 1)
    );
    let b1 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Deallocate,
            arguments: list![const_bool(true), const_int::<usize>(4), const_int::<usize>(4)], // bool unexpected here
            ret: None,
            next_block: Some(BbName(Name::from_internal(2))),
        },
    );
    let b2 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "invalid first argument to `Intrinsic::Deallocate`, not a pointer");
}

#[test]
fn dealloc_wrongarg2() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        allocate(const_int::<usize>(4), const_int::<usize>(4), local(0), 1)
    );
    let b1 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Deallocate,
            arguments: list![load(local(0)), const_bool(true), const_int::<usize>(4)], // bool unexpected here
            ret: None,
            next_block: Some(BbName(Name::from_internal(2))),
        },
    );
    let b2 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "invalid second argument to `Intrinsic::Deallocate`, not an integer");
}

#[test]
fn dealloc_wrongarg3() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        allocate(const_int::<usize>(4), const_int::<usize>(4), local(0), 1)
    );
    let b1 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Deallocate,
            arguments: list![load(local(0)), const_int::<usize>(4), const_bool(true)], // bool unexpected here
            ret: None,
            next_block: Some(BbName(Name::from_internal(2))),
        },
    );
    let b2 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "invalid third argument to `Intrinsic::Deallocate`, not an integer");
}

#[test]
fn dealloc_wrongreturn() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        allocate(const_int::<usize>(4), const_int::<usize>(4), local(0), 1)
    );
    let b1 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Deallocate,
            arguments: list![load(local(0)), const_int::<usize>(4), const_int::<usize>(4)],
            ret: Some(local(0)),
            next_block: Some(BbName(Name::from_internal(2))),
        },
    );
    let b2 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "invalid return type for `Intrinsic::Deallocate`");
}
