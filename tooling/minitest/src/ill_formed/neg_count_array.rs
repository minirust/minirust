use crate::*;

#[test]
fn neg_count_array() {
    let ty = array_ty(<()>::get_type(), -1);
    let pty = ptype(ty, align(1));
    let locals = &[
        pty,
    ];

    let stmts = &[ storage_live(0) ];

    let p = small_program(locals, stmts);
    dump_program(p);
    assert_ill_formed(p);
}
