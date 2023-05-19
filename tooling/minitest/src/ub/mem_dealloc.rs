use crate::*;

// this tests mem/basic.md

#[test]
fn mem_dealloc_success() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        allocate(const_int::<usize>(4), const_int::<usize>(4), local(0), 1)
    );
    let b1 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Deallocate,
            arguments: list![load(local(0)), const_int::<usize>(4), const_int::<usize>(4)],
            ret: None,
            next_block: Some(BbName(Name::from_internal(2))),
        },
    );
    let b2 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);
    let p = program(&[f]);
    dump_program(p);
    assert_stop(p);
}

#[test]
fn mem_dealloc_wrong_size() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        allocate(const_int::<usize>(4), const_int::<usize>(4), local(0), 1)
    );
    let b1 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Deallocate,
            arguments: list![load(local(0)), const_int::<usize>(5), const_int::<usize>(4)],
            ret: None,
            next_block: Some(BbName(Name::from_internal(2))),
        },
    );
    let b2 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "deallocating with incorrect size information");
}

#[test]
fn mem_dealloc_wrong_align() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        allocate(const_int::<usize>(4), const_int::<usize>(4), local(0), 1)
    );
    let b1 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Deallocate,
            arguments: list![load(local(0)), const_int::<usize>(4), const_int::<usize>(8)],
            ret: None,
            next_block: Some(BbName(Name::from_internal(2))),
        },
    );
    let b2 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "deallocating with incorrect alignment information");
}

#[test]
// this tests generates a dangling ptr by casting the int 42 to a pointer.
// then we deallocate the ptr, to obtain UB.
fn mem_dealloc_inv_ptr() {
    let union_ty = union_ty(&[
            (size(0), <usize>::get_type()),
            (size(0), <*const i32>::get_type()),
        ], size(8));
    let union_pty = ptype(union_ty, align(8));

    let locals = [ union_pty ];

    let b0 = block!(
        storage_live(0),
        assign(
            field(local(0), 0),
            const_int::<usize>(42)
        ),
        deallocate(
            load(field(local(0), 1)),
            const_int::<usize>(4), // size
            const_int::<usize>(4), // align
            1,
        )
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "deallocating invalid pointer");
}


#[test]
fn mem_dealloc_not_beginning() {
    let locals = [ <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        allocate(const_int::<usize>(4), const_int::<usize>(4), local(0), 1)
    );
    let b1 = block!(
        assign(
            local(0),
            ptr_offset(
                load(local(0)),
                const_int::<usize>(1),
                InBounds::Yes
            )
        ),
        deallocate(
            load(local(0)),
            const_int::<usize>(4),
            const_int::<usize>(4),
            2
        )
    );
    let b2 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "deallocating with pointer not to the beginning of its allocation");
}

#[test]
fn double_free() {
    let locals = vec![<*const i32>::get_ptype()];
    let n = const_int::<usize>(4);
    let b0 = block!(storage_live(0), allocate(n, n, local(0), 1));
    let b1 = block!(deallocate(load(local(0)), n, n, 2));
    let b2 = block!(deallocate(load(local(0)), n, n, 3));
    let b3 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1, b2, b3]);
    let p = program(&[f]);
    assert_ub(p, "double-free");
}
