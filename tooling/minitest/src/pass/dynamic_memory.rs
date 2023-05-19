use crate::*;

#[test]
fn dynamic_memory() {
    let locals = [<*const i32>::get_ptype(), <i32>::get_ptype()];
    let n = const_int::<usize>(4);
    let b0 = block!(storage_live(0), storage_live(1), allocate(n, n, local(0), 1)); // alloc ptr
    let b1 = block!(
        assign( // write to ptr
            deref(load(local(0)), <i32>::get_ptype()),
            const_int::<i32>(42),
        ),
        assign( // read from ptr
            local(1),
            load(deref(load(local(0)), <i32>::get_ptype())),
        ),
        deallocate(load(local(0)), n, n, 2)
    );
    let b2 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);
    let p = program(&[f]);
    assert_stop(p);
}
