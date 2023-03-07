use crate::*;

#[test]
fn dead_before_live() {
    let locals = vec![ <bool>::get_ptype() ];
    let stmts = vec![dead(0)];
    let p = small_program(&locals, &stmts);
    assert_ill_formed(p);
}
