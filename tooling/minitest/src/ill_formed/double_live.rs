use crate::*;

#[test]
fn double_live() {
    let locals = vec![ <bool>::get_ptype() ];
    let stmts = vec![live(0), live(0)];
    let p = small_program(&locals, &stmts);
    assert_ill_formed(p);
}
