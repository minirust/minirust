use crate::*;

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
        live(0),
        live(1),
        assign(
            local(1),
            load(field(local(0), 1)),
        ),
    ];

    let p = small_program(locals, stmts);
    dump_program(&p);
    assert_stop(p);
}
