use crate::*;

// Tests for Acquire

#[test]
fn acquire_arg_count() {
    let locals = [];

    let b0 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Lock(LockIntrinsic::Acquire),
            arguments: list![],
            ret: None,
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );

    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);

    assert_ub(p, "invalid number of arguments for `LockIntrinsic::Acquire`")
}

#[test]
fn acquire_arg_value() {
    let locals = [<()>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        acquire(load(local(0)), 1),
    );

    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);

    assert_ub(p, "invalid first argument to `LockIntrinsic::Acquire`")
}

#[test]
fn acquire_wrongreturn() {
    let locals = [<u32>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Lock(LockIntrinsic::Acquire),
            arguments: list![const_int::<u32>(0)],
            ret: Some(local(0)),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );

    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);

    assert_ub(p, "invalid return type for `LockIntrinsic::Acquire`")
}

#[test]
fn acquire_non_existent() {
    let locals = [<u32>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        assign(local(0), const_int::<u32>(0)),
        acquire(load(local(0)), 1),
    );

    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);

    assert_ub(p, "acquiring non-existing lock")
}

// Tests for Release

#[test]
fn release_arg_count() {
    let locals = [<()>::get_ptype()];

    let b0 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Lock(LockIntrinsic::Release),
            arguments: list![],
            ret: None,
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );

    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);

    assert_ub(p, "invalid number of arguments for `LockIntrinsic::Release`")
}

#[test]
fn release_arg_value() {
    let locals = [<()>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        release(load(local(0)), 1),
    );

    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);

    assert_ub(p, "invalid first argument to `LockIntrinsic::Release`")
}

#[test]
fn release_wrongreturn() {
    let locals = [<u32>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Lock(LockIntrinsic::Release),
            arguments: list![const_int::<u32>(0)],
            ret: Some(local(0)),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );

    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);

    assert_ub(p, "invalid return type for `LockIntrinsic::Release`")
}

#[test]
fn release_non_existent() {
    let locals = [<u32>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        assign(local(0), const_int::<u32>(0)),
        release(load(local(0)), 1),
    );

    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);

    assert_ub(p, "releasing non-existing lock")
}

#[test]
fn release_non_owned() {
    let locals = [<u32>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        create_lock(local(0), 1),
    );

    let b1 = block!(
        release(load(local(0)), 2),
    );

    let b2 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);

    let p = program(&[f]);

    assert_ub(p, "releasing non-acquired lock")
}

// Create lock

#[test]
fn create_arg_count() {
    let locals = [<()>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Lock(LockIntrinsic::Create),
            arguments: list![load(local(0))],
            ret: None,
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );

    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);

    assert_ub(p, "invalid number of arguments for `LockIntrinsic::Create`")
}

#[test]
fn create_wrongreturn() {
    let locals = [<()>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Lock(LockIntrinsic::Create),
            arguments: list![],
            ret: Some(local(0)),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );

    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);

    assert_ub(p, "invalid return type for `LockIntrinsic::Create`")
}
