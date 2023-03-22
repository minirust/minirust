use crate::*;

#[test]
fn invalid_offset() {
    let union_ty = union_ty(&[
            (size(0), <*const i32>::get_type()),
            (size(0), <usize>::get_type()),
        ], size(8));
    let union_pty = ptype(union_ty, align(8));
    let locals = &[
        <[i32; 2]>::get_ptype(),
        union_pty
    ];

    let stmts = &[
        storage_live(0),
        storage_live(1),
        assign(local(0),
            const_array(&[
                const_int::<i32>(42),
                const_int::<i32>(24),
            ], <i32>::get_type()),
        ),
        assign(
            field(local(1), 0),
            addr_of(index(local(0), const_int::<usize>(0)), <*const i32>::get_type())
        ),
        assign( // strips provenance!
            field(local(1), 1),
            load(field(local(1), 1)),
        ),
        assign(
            field(local(1), 0),
            ptr_offset(
                load(field(local(1), 0)),
                const_int::<usize>(4),
                InBounds::Yes,
            )
        ),
    ];

    let p = small_program(locals, stmts);
    dump_program(p);
    assert_ub(p, "non-zero-sized access with invalid pointer");
}
