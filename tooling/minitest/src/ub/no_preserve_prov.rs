use crate::*;

// see https://github.com/rust-lang/miri/issues/2182
#[test]
fn no_preserve_prov() {
    let union_ty = union_ty(&[
            (size(0), <[&i32; 1]>::get_type()),
            (size(0), <[usize; 1]>::get_type()),
            (size(0), <&i32>::get_type()),
        ], size(8));
    let union_pty = ptype(union_ty, align(8));

    let locals = vec![
        <i32>::get_ptype(),
        union_pty,
        <i32>::get_ptype(),
    ];

    let stmts = vec![
        storage_live(0),
        storage_live(1),
        storage_live(2),
        assign(local(0), const_int::<i32>(42)), // _0 = 42;
        assign( // _1.0[0] = &_0;
            index(
                field(local(1), 0),
                const_int::<usize>(0)
            ),
            addr_of(local(0), <&i32>::get_type()),
        ),
        assign( // _1.1 = load(_1.1); This re-writes itself as [usize; 1]. This strips provenance.
            field(local(1), 1),
            load(field(local(1), 1)),
        ),
        assign( // _2 = load(*load(_1.2))
            local(2),
            load(deref(
                load(field(local(1), 2)),
                <i32>::get_ptype(),
            ))
        ),
    ];

    let p = small_program(&locals, &stmts);
    dump_program(p);
    assert_ub(p, "non-zero-sized access with invalid pointer");
}
