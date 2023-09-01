use crate::*;

fn make_packed() -> Type {
    tuple_ty(
        &[(size(0), <i32>::get_type())],
        size(4),
        align(1),
    )
}

#[test]
fn packed_works() {
    let locals = [make_packed(), <i32>::get_type()];
    let b0 = block!(
        storage_live(0),
        assign(
            field(local(0), 0),
            const_int(0i32),
        ),
        storage_live(1),
        assign(
            local(1),
            load(field(local(0), 0)),
        ),
        exit(),
    );
    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    assert_stop(p);
}

#[test]
fn packed_is_not_aligned() {
    let locals = [make_packed(), <&i32>::get_type()];
    let b0 = block!(
        storage_live(0),
        assign(
            field(local(0), 0),
            const_int(0i32),
        ),
        storage_live(1),
        assign(
            local(1),
            addr_of(field(local(0), 0), <&i32>::get_type()),
        ),
        exit(),
    );
    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    assert_ub_eventually(p, 16, "taking the address of an invalid (null, misaligned, or uninhabited) place");
}
