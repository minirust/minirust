use crate::*;

#[test]
fn double_live() {
    let locals = vec![ <bool>::get_ptype() ];
    let stmts = vec![storage_live(0), storage_live(0)];
    let p = small_program(&locals, &stmts);
    assert_ill_formed(p);
}
