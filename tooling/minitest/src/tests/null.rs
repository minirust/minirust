use crate::*;

#[test]
fn ptr_null() {
    let union_ty = union_ty(&[
            (size(0), <usize>::get_type()),
            (size(0), <*const i32>::get_type()),
        ], size(8), align(8));

    let locals = [ union_ty, <i32>::get_type(), ];

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(
            field(local(0), 0),
            const_int::<usize>(0) // nullptr!
        ),
        assign(
            local(1),
            load(deref(load(field(local(0), 1)), <i32>::get_type()))
        ),
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "memory access with null pointer");
}

#[test]
fn ptr_null_zst() {
    let union_ty = union_ty(&[
            (size(0), <usize>::get_type()),
            (size(0), <*const ()>::get_type()),
        ], size(8), align(8));

    let locals = [ union_ty, <()>::get_type(), ];

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(
            field(local(0), 0),
            const_int::<usize>(0) // nullptr!
        ),
        assign(
            local(1),
            load(deref(load(field(local(0), 1)), <()>::get_type()))
        ),
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "memory access with null pointer");
}
