use crate::*;

#[test]
/// What this test wants to check is wether there can be a data race
/// after a lock handover.
/// 
/// let global_0 = 0;
/// let global_1 = null;
/// 
/// fn critical() {
///     acquire(global_0);
///     atomic_write(global_1, &global_0);
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
fn lock_issue() {
    let locals = [<()>::get_ptype()];

    let ptr_ty = <*const u32>::get_type();

    let p_ptype = <u32>::get_ptype();
    
    let b0 = block!(
        acquire(load(global::<u32>(0)), 1) 
    );
    let b1 = block!( atomic_write(addr_of(global::<*const u32>(1), <*const *const u32>::get_type()), addr_of(global::<u32>(0), ptr_ty), 2));
    let b2 = block!( release(load(deref(load(global::<*const u32>(1)), p_ptype)), 3) );
    let b3 = block!( return_() );
    let critical = function(Ret::Yes, 0, &locals, &[b0,b1,b2,b3]);


    let locals = [<u32>::get_ptype(), <()>::get_ptype()];
    
    let b0 = block!(
        storage_live(0),
        storage_live(1),
        create_lock(global::<u32>(0), 1),
    );
    let b1 = block!( spawn(fn_ptr(1), null(), Some(local(0)), 2) );
    let b2 = block!( call(2, &[], Some(local(1)), Some(3)));
    let b3 = block!( join(load(local(0)), 4) );
    let b4 = block!( exit() );
    let main = function(Ret::No, 0, &locals, &[b0,b1,b2,b3,b4]);
    
    let locals = [<()>::get_ptype(), <*const ()>::get_ptype()];

    let b0 = block!(
        call(2, &[], Some(local(0)), Some(1))
    );
    let b1 = block!( return_() );
    let second = function(Ret::Yes, 1, &locals, &[b0,b1]);

    let globals = [global_int::<u32>(), global_ptr::<u32>()];

    let p = program_with_globals(&[main, second, critical], &globals);
    assert_eq!(run_program(p), TerminationInfo::MachineStop);
}
