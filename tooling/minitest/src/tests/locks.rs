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
    let mut p = ProgramBuilder::new();
    let lock = p.declare_global_zero_initialized::<u32>();

    let critical: FnName = {
        let mut critical = p.declare_function();
        let val = critical.declare_local::<u32>();

        critical.storage_live(val);
        critical.assign(val, const_int(0u32));
        critical.lock_acquire(load(lock));
        critical.while_(ne(load(val), const_int(256_u32)), |f| {
            f.assign(val, add(load(val), const_int(1u32)));
        });
        critical.lock_release(load(lock));
        critical.return_();

        p.finish_function(critical)
    };

    let mut second = p.declare_function();

    let main: FnName = {
        let mut main = p.declare_function();
        let thread_id = main.declare_local::<u32>();

        main.storage_live(thread_id);
        main.lock_create(lock);
        main.spawn(second.name(), null(), thread_id);
        main.call_ignoreret(critical, &[]);
        main.join(load(thread_id));
        main.exit();

        p.finish_function(main)
    };

    // implement function `second`
    {
        second.declare_arg::<*const ()>();
        second.call_ignoreret(critical, &[]);
        second.return_();
        p.finish_function(second);
    }

    let p = p.finish_program(main);
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
    let mut p = ProgramBuilder::new();
    let lock = p.declare_global_zero_initialized::<u32>();
    let gstore = p.declare_global_zero_initialized::<*const u32>();

    let critical: FnName = {
        let mut critical = p.declare_function();

        critical.lock_acquire(load(lock));
        critical.atomic_store(
            addr_of(gstore, <*const *const u32>::get_type()),
            addr_of(lock, <*const u32>::get_type()),
        );
        critical.lock_release(load(deref(load(gstore), <u32>::get_type())));
        critical.return_();

        p.finish_function(critical)
    };

    let mut second = p.declare_function();

    let main: FnName = {
        let mut main = p.declare_function();
        let thread_id = main.declare_local::<u32>();

        main.storage_live(thread_id);
        main.lock_create(lock);
        main.spawn(second.name(), null(), thread_id);
        main.call_ignoreret(critical, &[]);
        main.join(load(thread_id));
        main.exit();

        p.finish_function(main)
    };

    // implement function `second`
    {
        second.declare_arg::<*const ()>();
        second.call_ignoreret(critical, &[]);
        second.return_();
        p.finish_function(second);
    }

    let p = p.finish_program(main);
    assert_stop_always(p, 10);
}

// UB Tests for Acquire

#[test]
fn acquire_arg_count() {
    let b0 = block!(Terminator::Intrinsic {
        intrinsic: IntrinsicOp::Lock(IntrinsicLockOp::Acquire),
        arguments: list![],
        ret: zst_place(),
        next_block: Some(BbName(Name::from_internal(1))),
    });
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &[], &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid number of arguments for `Acquire` lock intrinsic")
}

#[test]
fn acquire_arg_value() {
    let locals = [<()>::get_type()];

    let b0 = block!(storage_live(0), lock_acquire(load(local(0)), 1),);
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid first argument to `Acquire` lock intrinsic")
}

#[test]
fn acquire_wrongreturn() {
    let locals = [<u32>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::Intrinsic {
            intrinsic: IntrinsicOp::Lock(IntrinsicLockOp::Acquire),
            arguments: list![const_int::<u32>(0)],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid return type for `Acquire` lock intrinsic")
}

#[test]
fn acquire_non_existent() {
    let locals = [<u32>::get_type()];

    let b0 = block!(
        storage_live(0),
        assign(local(0), const_int::<u32>(0)),
        lock_acquire(load(local(0)), 1),
    );
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "acquiring non-existing lock")
}

// UB Tests for Release

#[test]
fn release_arg_count() {
    let locals = [<()>::get_type()];

    let b0 = block!(Terminator::Intrinsic {
        intrinsic: IntrinsicOp::Lock(IntrinsicLockOp::Release),
        arguments: list![],
        ret: zst_place(),
        next_block: Some(BbName(Name::from_internal(1))),
    });
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid number of arguments for `Release` lock intrinsic")
}

#[test]
fn release_arg_value() {
    let locals = [<()>::get_type()];

    let b0 = block!(storage_live(0), lock_release(load(local(0)), 1),);
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid first argument to `Release` lock intrinsic")
}

#[test]
fn release_wrongreturn() {
    let locals = [<u32>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::Intrinsic {
            intrinsic: IntrinsicOp::Lock(IntrinsicLockOp::Release),
            arguments: list![const_int::<u32>(0)],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid return type for `Release` lock intrinsic")
}

#[test]
fn release_non_existent() {
    let locals = [<u32>::get_type()];

    let b0 = block!(
        storage_live(0),
        assign(local(0), const_int::<u32>(0)),
        lock_release(load(local(0)), 1),
    );
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "releasing non-existing lock")
}

#[test]
fn release_non_owned() {
    let locals = [<u32>::get_type()];

    let b0 = block!(storage_live(0), lock_create(local(0), 1),);
    let b1 = block!(lock_release(load(local(0)), 2),);
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
        Terminator::Intrinsic {
            intrinsic: IntrinsicOp::Lock(IntrinsicLockOp::Create),
            arguments: list![load(local(0))],
            ret: zst_place(),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid number of arguments for `Create` lock intrinsic")
}

#[test]
fn create_wrongreturn() {
    let locals = [<()>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::Intrinsic {
            intrinsic: IntrinsicOp::Lock(IntrinsicLockOp::Create),
            arguments: list![],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid return type for `Create` lock intrinsic")
}

// Other errors

#[test]
fn deadlock() {
    // The main function creates a lock and acquires it.
    // The second function then tries to get this lock while the main tries to join the second thread.
    // In such a situation both threads wait for each other and we have a deadlock.

    let mut p = ProgramBuilder::new();
    let lock = p.declare_global_zero_initialized::<u32>();

    let mut second = p.declare_function();

    let main: FnName = {
        let mut main = p.declare_function();
        let thread_id = main.declare_local::<u32>();

        main.lock_create(lock);
        main.lock_acquire(load(lock));
        main.storage_live(thread_id);
        main.spawn(second.name(), null(), thread_id);
        main.join(load(thread_id));
        main.lock_release(load(lock));
        main.exit();

        p.finish_function(main)
    };

    // implement function `second`
    {
        second.declare_arg::<*const ()>();
        second.lock_acquire(load(lock));
        second.lock_release(load(lock));
        second.return_();
        p.finish_function(second);
    }

    let p = p.finish_program(main);
    assert_deadlock(p);
}
