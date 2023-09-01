use crate::*;

#[test]
fn main_returns() {
    let b0 = block!(
        return_(),
    );
    let f = function(Ret::No, 0, &[], &[b0]);
    let p = program(&[f]);
    assert_ub(p, "the start function must not return");
}
