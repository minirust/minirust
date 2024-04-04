use crate::*;

/// Test if `expose` called with non-pointer is ill-formed.
#[test]
fn requires_pointer() {
    let locals = [];
    let blocks = [block!(expose(const_bool(false)), exit())];

    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_ill_formed(program);
}

/// Test if `expose` called with pointer works.
#[test]
fn pointer_works() {
    let locals = [<i32>::get_type(), <*const i32>::get_type()];
    let blocks = [block!(
        storage_live(0),
        assign(local(0), const_int::<i32>(42)),
        storage_live(1),
        assign(local(1), addr_of(local(1), <*const i32>::get_type()),),
        expose(load(local(1))),
        storage_dead(1),
        storage_dead(0),
        exit(),
    )];

    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop(program);
}
