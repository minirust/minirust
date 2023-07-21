use crate::*;

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
    let locals = [<()>::get_ptype(), <u32>::get_ptype()];

    let b0 = block!(
        storage_live(1),
        assign(local(1), const_int::<u32>(0)),
        acquire(load(global::<u32>(0)), 1) 
    );
    let b1 = block!( if_(eq(load(local(1)), const_int::<u32>(256)), 3, 2) );
    let b2 = block!(
        assign(local(1), add::<u32>(load(local(1)), const_int::<u32>(1))),
        goto(1)
    );
    let b3 = block!( release(load(global::<u32>(0)), 4) );
    let b4 = block!( return_() );
    let critical = function(Ret::Yes, 0, &locals, &[b0,b1,b2,b3,b4]);


    let locals = [<u32>::get_ptype(), <()>::get_ptype()];
    
    let b0 = block!(
        storage_live(0),
        storage_live(1),
        create_lock(global::<u32>(0), 1),
    );
    let b1 = block!( spawn(fn_ptr(1), Some(local(0)), 2) );
    let b2 = block!( call(2, &[], Some(local(1)), Some(3)));
    let b3 = block!( join(load(local(0)), 4) );
    let b4 = block!( exit() );
    let main = function(Ret::No, 0, &locals, &[b0,b1,b2,b3,b4]);
    
    let locals = [<()>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        call(2, &[], Some(local(0)), Some(1))
    );
    let b1 = block!( return_() );
    let second = function(Ret::No, 0, &locals, &[b0,b1]);

    let globals = [global_int::<u32>()];

    let p = program_with_globals(&[main, second, critical], &globals);
    assert_eq!(run_program(p), TerminationInfo::MachineStop);
}
