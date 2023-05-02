use crate::*;

fn mk_main_fn() -> Function {
    let b = block!(call(1, &[], None, None));
    function(Ret::No, 0, &[], &[b])
}

fn mk_never_fn() -> Function {
    let b = block!(return_());
    function(Ret::No, 0, &[], &[b])
}

#[test]
fn return_from_never_fn() {
    let p = program(&[mk_main_fn(), mk_never_fn()], &[]);
    assert_ub(p, "return from a function that does not have a return local");
}
