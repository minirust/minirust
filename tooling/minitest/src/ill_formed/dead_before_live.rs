use crate::*;

#[test]
fn dead_before_live() {
    let locals = vec![ <bool>::get_ptype() ];
    let stmts = vec![storage_dead(0)];
    let p = small_program(&locals, &stmts);
    assert_ill_formed(p);
}
