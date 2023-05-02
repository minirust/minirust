use crate::*;

#[test]
fn ptr_offset_success() {
    let locals = &[ <i32>::get_ptype(), <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(
            local(0),
            const_int::<i32>(42),
        ),
        assign(
            local(1),
            addr_of(local(0), <*const i32>::get_type())
        ),
        assign(
            local(1),
            ptr_offset(
                load(local(1)),
                const_int::<usize>(4),
                InBounds::Yes,
            )
        ),
        exit()
    );

    let f = function(Ret::No, 0, locals, &[b0]);
    let p = program(&[f], &[]);
    dump_program(p);
    assert_stop(p);
}

#[test]
fn ptr_offset_inbounds() {
    let locals = &[ <i32>::get_ptype(), <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(
            local(0),
            const_int::<i32>(42),
        ),
        assign(
            local(1),
            addr_of(local(0), <*const i32>::get_type())
        ),
        assign(
            local(1),
            ptr_offset(
                load(local(1)),
                const_int::<usize>(usize::MAX),
                InBounds::Yes,
            )
        ),
        exit()
    );

    let f = function(Ret::No, 0, locals, &[b0]);
    let p = program(&[f], &[]);
    dump_program(p);
    assert_ub(p, "inbounds offset does not fit into `isize`");
}

#[test]
fn ptr_offset_no_inbounds() {
    let locals = &[ <i32>::get_ptype(), <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(
            local(0),
            const_int::<i32>(42),
        ),
        assign(
            local(1),
            addr_of(local(0), <*const i32>::get_type())
        ),
        assign(
            local(1),
            ptr_offset(
                load(local(1)),
                const_int::<usize>(usize::MAX), // this huge offset is out of range, but InBounds::No cannot fail.
                InBounds::No,
            )
        ),
        exit()
    );

    let f = function(Ret::No, 0, locals, &[b0]);
    let p = program(&[f], &[]);
    dump_program(p);
    assert_stop(p);
}

#[test]
fn ptr_offset_overflow() {
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
            const_int::<usize>(usize::MAX) // this is the largest possible pointer.
        ),
        assign(
            field(local(0), 1),
            ptr_offset( // here we add 1 to the largest possible pointer -> overflow.
                load(field(local(0), 1)),
                const_int::<usize>(1),
                InBounds::Yes
            ),
        ),
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f], &[]);
    dump_program(p);
    assert_ub(p, "overflowing inbounds pointer arithmetic");
}


#[test]
fn ptr_offset_out_of_bounds() {
    let locals = &[ <i32>::get_ptype(), <*const i32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(
            local(0),
            const_int::<i32>(42),
        ),
        assign(
            local(1),
            addr_of(local(0), <*const i32>::get_type())
        ),
        assign(
            local(1),
            ptr_offset(
                load(local(1)),
                const_int::<usize>(5), // an offset of 5 is too large for an allocation of 4 bytes!
                InBounds::Yes,
            )
        ),
        exit()
    );

    let f = function(Ret::No, 0, locals, &[b0]);
    let p = program(&[f], &[]);
    dump_program(p);
    assert_ub(p, "out-of-bounds memory access");
}
