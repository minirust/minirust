use crate::*;

#[test]
fn uninit_read() {
    let locals = vec![ <bool>::get_ptype(); 2];
    let stmts = vec![
        live(0),
        live(1),
        assign(
            local(0),
            load(local(1)),
        ),
    ];
    let p = small_program(&locals, &stmts);
    assert_ub(p, "load at type PlaceType { ty: Bool, align: Align { raw: Int(Small(1)) } } but the data in memory violates the validity invariant");
}
