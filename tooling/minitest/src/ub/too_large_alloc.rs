use crate::*;

#[test]
fn too_large_alloc() {
    let locals = vec![<usize>::get_ptype()];
    let b = block(
        &[live(0)],
        allocate(const_int::<usize>(usize::MAX / 2 + 1), const_int::<usize>(1), local(0), 1),
    );
    let b2 = block(&[], exit());
    let f = function(Ret::No, 0, &locals, &[b, b2]);
    let p = program(&[f]);
    assert_ub(p, "asking for a too large allocation");
}
