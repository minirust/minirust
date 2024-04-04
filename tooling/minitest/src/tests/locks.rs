use crate::*;

// Passing tests

#[test]
/// This test is written in response to a bug that was found.
/// The bug was: When handing over a lock instead of making the lock owned by the acquiring thread
/// it was once again owned by the releasing thread.
/// This lead to UB once the acquirer tried to release the lock.
///
/// What it wants to check is: Does the lock handover work correctly?
/// By making the critical section large (256 times around a loop)
/// we get a high probability that the handover happened.
fn lock_handover() {
    let locals = [<()>::get_type(), <u32>::get_type()];

    let b0 = block!(
        storage_live(1),
        assign(local(1), const_int::<u32>(0)),
        acquire(load(global::<u32>(0)), 1)
    );
    let b1 = block!(if_(eq(load(local(1)), const_int::<u32>(256)), 3, 2));
    let b2 = block!(assign(local(1), add(load(local(1)), const_int::<u32>(1))), goto(1));
    let b3 = block!(release(load(global::<u32>(0)), 4));
    let b4 = block!(return_());
    let critical = function(Ret::Yes, 0, &locals, &[b0, b1, b2, b3, b4]);

    let locals = [<u32>::get_type(), <()>::get_type()];

    let b0 = block!(storage_live(0), storage_live(1), create_lock(global::<u32>(0), 1),);
    let b1 = block!(spawn(fn_ptr(1), null(), local(0), 2));
    let b2 = block!(call(2, &[], local(1), Some(3)));
    let b3 = block!(join(load(local(0)), 4));
    let b4 = block!(exit());
    let main = function(Ret::No, 0, &locals, &[b0, b1, b2, b3, b4]);

    let locals = [<()>::get_type(), <*const ()>::get_type()];

    let b0 = block!(call(2, &[], local(0), Some(1)));
    let b1 = block!(return_());
    let second = function(Ret::Yes, 1, &locals, &[b0, b1]);

    let globals = [global_int::<u32>()];

    let p = program_with_globals(&[main, second, critical], &globals);
    assert_stop(p);
}

#[test]
/// What this test wants to check is wether there can be a data race
/// after a lock handover.
///
/// let global_0 = 0;
/// let global_1 = null;
///
/// fn critical() {
///     acquire(global_0);
///     atomic_store(global_1, &global_0);
///     release(*global_1)
/// }
///
/// fn second() {
///     critical();
/// }
///
/// fn main() {
///     global_0 = create_lock();
///     let id = spawn(second, null);
///     critical();
///     join(id);
/// }
///
/// If a handover occurs and data race detection does not synchronize the acquirer,
/// it immediatly writing to global_1 would be a data race with the release.
fn lock_handover_data_race() {
    let locals = [<()>::get_type()];

    let ptr_ty = <*const u32>::get_type();

    let p_ptype = <u32>::get_type();

    let b0 = block!(acquire(load(global::<u32>(0)), 1));
    let b1 = block!(atomic_store(
        addr_of(global::<*const u32>(1), <*const *const u32>::get_type()),
        addr_of(global::<u32>(0), ptr_ty),
        2
    ));
    let b2 = block!(release(load(deref(load(global::<*const u32>(1)), p_ptype)), 3));
    let b3 = block!(return_());
    let critical = function(Ret::Yes, 0, &locals, &[b0, b1, b2, b3]);

    let locals = [<u32>::get_type(), <()>::get_type()];

    let b0 = block!(storage_live(0), storage_live(1), create_lock(global::<u32>(0), 1),);
    let b1 = block!(spawn(fn_ptr(1), null(), local(0), 2));
    let b2 = block!(call(2, &[], local(1), Some(3)));
    let b3 = block!(join(load(local(0)), 4));
    let b4 = block!(exit());
    let main = function(Ret::No, 0, &locals, &[b0, b1, b2, b3, b4]);

    let locals = [<()>::get_type(), <*const ()>::get_type()];

    let b0 = block!(call(2, &[], local(0), Some(1)));
    let b1 = block!(return_());
    let second = function(Ret::Yes, 1, &locals, &[b0, b1]);

    let globals = [global_int::<u32>(), global_ptr::<u32>()];

    let p = program_with_globals(&[main, second, critical], &globals);
    assert_stop_always(p, 10);
}

// UB Tests for Acquire

#[test]
fn acquire_arg_count() {
    let b0 = block!(Terminator::CallIntrinsic {
        intrinsic: Intrinsic::Lock(LockIntrinsic::Acquire),
        arguments: list![],
        ret: zst_place(),
        next_block: Some(BbName(Name::from_internal(1))),
    });
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &[], &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid number of arguments for `LockIntrinsic::Acquire`")
}

#[test]
fn acquire_arg_value() {
    let locals = [<()>::get_type()];

    let b0 = block!(storage_live(0), acquire(load(local(0)), 1),);
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid first argument to `LockIntrinsic::Acquire`")
}

#[test]
fn acquire_wrongreturn() {
    let locals = [<u32>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Lock(LockIntrinsic::Acquire),
            arguments: list![const_int::<u32>(0)],
            ret: local(0),
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
    let locals = [<u32>::get_type()];

    let b0 =
        block!(storage_live(0), assign(local(0), const_int::<u32>(0)), acquire(load(local(0)), 1),);
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "acquiring non-existing lock")
}

// UB Tests for Release

#[test]
fn release_arg_count() {
    let locals = [<()>::get_type()];

    let b0 = block!(Terminator::CallIntrinsic {
        intrinsic: Intrinsic::Lock(LockIntrinsic::Release),
        arguments: list![],
        ret: zst_place(),
        next_block: Some(BbName(Name::from_internal(1))),
    });
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid number of arguments for `LockIntrinsic::Release`")
}

#[test]
fn release_arg_value() {
    let locals = [<()>::get_type()];

    let b0 = block!(storage_live(0), release(load(local(0)), 1),);
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid first argument to `LockIntrinsic::Release`")
}

#[test]
fn release_wrongreturn() {
    let locals = [<u32>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Lock(LockIntrinsic::Release),
            arguments: list![const_int::<u32>(0)],
            ret: local(0),
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
    let locals = [<u32>::get_type()];

    let b0 =
        block!(storage_live(0), assign(local(0), const_int::<u32>(0)), release(load(local(0)), 1),);
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "releasing non-existing lock")
}

#[test]
fn release_non_owned() {
    let locals = [<u32>::get_type()];

    let b0 = block!(storage_live(0), create_lock(local(0), 1),);
    let b1 = block!(release(load(local(0)), 2),);
    let b2 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);

    let p = program(&[f]);
    assert_ub(p, "releasing non-acquired lock")
}

// UB on Create lock

#[test]
fn create_arg_count() {
    let locals = [<()>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Lock(LockIntrinsic::Create),
            arguments: list![load(local(0))],
            ret: zst_place(),
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
    let locals = [<()>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Lock(LockIntrinsic::Create),
            arguments: list![],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid return type for `LockIntrinsic::Create`")
}

// Other errors

#[test]
fn deadlock() {
    // The main function creates a lock and acquires it.
    // The second function then tries to get this lock while the main tries to join the second thread.
    // In such a situation both threads wait for each other and we have a deadlock.

    // The locals are used to store the thread ids.
    let locals = [<u32>::get_type()];

    let b0 = block!(create_lock(global::<u32>(0), 1));
    let b1 = block!(acquire(load(global::<u32>(0)), 2));
    let b2 = block!(storage_live(0), spawn(fn_ptr(1), null(), local(0), 3));
    let b3 = block!(join(load(local(0)), 4));
    let b4 = block!(release(load(global::<u32>(0)), 5));
    let b5 = block!(exit());
    let main = function(Ret::No, 0, &locals, &[b0, b1, b2, b3, b4, b5]);

    let locals = [<()>::get_type(), <*const ()>::get_type()];
    let b0 = block!(acquire(load(global::<u32>(0)), 1));
    let b1 = block!(release(load(global::<u32>(0)), 2));
    let b2 = block!(return_());
    let second = function(Ret::Yes, 1, &locals, &[b0, b1, b2]);

    // global(0) is used as a lock. We store the lock id there.
    let globals = [global_int::<u32>()];

    let p = program_with_globals(&[main, second], &globals);

    assert_deadlock(p);
}
