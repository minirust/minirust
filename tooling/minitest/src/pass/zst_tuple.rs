use crate::*;

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
