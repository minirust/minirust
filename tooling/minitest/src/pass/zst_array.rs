use crate::*;

#[test]
fn zst_array() {
    let a = array_ty(<()>::get_type(), 2);
    let a_pty = ptype(a, align(1));
    let locals = &[
        a_pty,
    ];

    let stmts = &[
        live(0),
        assign(
            local(0),
            load(local(0))
        ),
    ];

    let p = small_program(locals, stmts);
    dump_program(&p);
    assert_stop(p);
}
