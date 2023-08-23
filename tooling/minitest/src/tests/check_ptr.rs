use crate::*;

#[test]
fn check_ptr_null() {
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
    assert_ub(p, "non-zero-sized access with invalid pointer");
}

#[test]
fn check_ptr_misaligned() {
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
            const_int::<usize>(1) // nullptr + 1
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
    assert_ub(p, "loading from a place based on a misaligned pointer");
}

#[test]
fn use_after_free() {
    let locals = vec![<*const i32>::get_type()];
    let n = const_int::<usize>(4);
    let b0 = block!(storage_live(0), allocate(n, n, local(0), 1));
    let b1 = block!(deallocate(load(local(0)), n, n, 2));
    let b2 = block!(
        assign( // write to ptr after dealloc!
            deref(load(local(0)), <i32>::get_type()),
            const_int::<i32>(42),
        ),
        exit()
    );
    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);
    let p = program(&[f]);
    assert_ub(p, "memory accessed after deallocation");
}
