use crate::*;

#[test]
fn zst_array() {
    let a = array_ty(<()>::get_type(), 2);
    let a_pty = ptype(a, align(1));
    let locals = &[
        a_pty,
    ];

    let stmts = &[
        storage_live(0),
        assign(
            local(0),
            load(local(0))
        ),
    ];

    let p = small_program(locals, stmts);
    dump_program(p);
    assert_stop(p);
}


#[test]
fn zst_tuple() {
    let tuple = tuple_ty(&[(size(0), <()>::get_type()); 2], size(0));
    let tuple_pty = ptype(tuple, align(1));
    let locals = &[
        tuple_pty,
        <()>::get_ptype(),
    ];

    let stmts = &[
        storage_live(0),
        storage_live(1),
        assign(
            local(1),
            load(field(local(0), 0)),
        ),
    ];

    let p = small_program(locals, stmts);
    dump_program(p);
    assert_stop(p);
}

#[test]
fn zst_tuple2() {
    let tuple = tuple_ty(&[
        (size(0), <i8>::get_type()),
        (size(1), <()>::get_type()),
    ], size(1));
    let tuple_pty = ptype(tuple, align(1));
    let locals = &[
        tuple_pty,
        <()>::get_ptype(),
    ];

    let stmts = &[
        storage_live(0),
        storage_live(1),
        assign(
            local(1),
            load(field(local(0), 1)),
        ),
    ];

    let p = small_program(locals, stmts);
    dump_program(p);
    assert_stop(p);
}
