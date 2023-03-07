use crate::*;

#[test]
fn reach_unreachable() {
    let locals = [];

    let b0 = block2(&[ &unreachable() ]);

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    dump_program(&p);
    assert_ub(p, "reached unreachable code");
}
