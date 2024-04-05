use crate::*;

/// Test if `expose` called with pointer works.
#[test]
fn pointer_works() {
    let locals = [<i32>::get_type(), <*const i32>::get_type(), <usize>::get_type()];
    let blocks = [
        block!(
            storage_live(0),
            assign(local(0), const_int::<i32>(42)),
            storage_live(1),
            assign(local(1), addr_of(local(1), <*const i32>::get_type())),
            storage_live(2),
            expose_provenance(local(2), load(local(1)), 1,)
        ),
        block!(storage_dead(2), storage_dead(1), storage_dead(0), exit()),
    ];

    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop(program);
}

/// Test if `expose` called with non-pointer is UB
#[test]
fn requires_pointer() {
    let locals = [<usize>::get_type()];
    let blocks = [
        block!(storage_live(0), expose_provenance(local(0), const_bool(true), 1,)),
        block!(storage_dead(0), exit()),
    ];

    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_ub(program, "invalid argument for `PointerExposeProvenance` intrinsic: not a pointer");
}
