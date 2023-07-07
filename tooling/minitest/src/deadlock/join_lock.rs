use crate::*;

#[test]
fn deadlock() {
    // The main function creates a lock and acquires it.
    // The second function then tries to get this lock while the main tries to join the second thread.
    // In such a situation both threads wait for each other and we have a deadlock.

    // The locals are used to store the thread ids.
    let locals = [<u32>::get_ptype()];

    let b0 = block!( create_lock(global::<u32>(0), 1) );
    let b1 = block!( acquire(load(global::<u32>(0)), 2) );
    let b2 = block!(
        storage_live(0),
        spawn(fn_ptr(1), Some(local(0)), 3)
    );
    let b3 = block!( join(load(local(0)), 4) );
    let b4 = block!( release(load(global::<u32>(0)), 5) );
    let b5 = block!( exit() );
    let main = function(Ret::No, 0, &locals, &[b0, b1, b2, b3, b4, b5]);

    let b0 = block!( acquire(load(global::<u32>(0)), 1) );
    let b1 = block!( release(load(global::<u32>(0)), 2) );
    let b2 = block!( return_() );
    let second = function(Ret::No, 0, &[], &[b0,b1,b2]);

    // global(0) is used as a lock. We store the lock id there.
    let globals = [global_int::<u32>()];

    let p = program_with_globals(&[main, second], &globals);

    assert_deadlock(p);
}
