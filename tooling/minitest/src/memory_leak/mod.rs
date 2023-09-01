use crate::*;

#[test]
fn memory_leak() {
    let locals = [<*mut i32>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        allocate(
            const_int::<usize>(1), // size
            const_int::<usize>(1), // align
            local(0),
            1,
        )
    );
    let b1 = block!(exit());
    let main = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[main]);
    assert_memory_leak(p);
}
