use crate::*;

#[test]
fn uninit_read() {
    let locals = vec![<bool>::get_type(); 2];
    let stmts = vec![storage_live(0), storage_live(1), assign(local(0), load(local(1)))];
    let p = small_program(&locals, &stmts);
    assert_ub::<BasicMem>(
        p,
        "load at type Bool but the data in memory violates the validity invariant",
    );
}
