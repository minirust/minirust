use crate::*;

#[test]
fn manual_align() {
    let locals = &[
        <[u8; 64]>::get_ptype(),
        <usize>::get_ptype()
    ];

    let stmts = &[
        storage_live(0),
        storage_live(1),
        assign( // _1 = (&raw _0) as usize;
            local(1),
            ptr_addr(
                addr_of(local(0), <*const u8>::get_type()),
            ),
        ),
        assign( // _1 = (8 + (_1 / 8 * 8)) - _1; This guarantees alignment of 8 for (&raw _0) + _1
            local(1),
            sub::<usize>(
                add::<usize>(
                    const_int::<usize>(8),
                    mul::<usize>(
                        div::<usize>(
                            load(local(1)),
                            const_int::<usize>(8)
                        ),
                        const_int::<usize>(8)
                    ),
                ),
                load(local(1))
            )
        ),
        assign(
            deref(
                ptr_offset(
                    addr_of(local(0), <*mut u64>::get_type()),
                    load(local(1)),
                    InBounds::Yes
                ),
                <u64>::get_ptype()
            ),
            const_int::<u64>(42)
        ),
    ];

    let p = small_program(locals, stmts);
    dump_program(p);
    assert_stop(p);
}

#[test]
// TODO this should not actually panic!
// However, this alignment makes allocation impossible, so `pick` has to give up and what else should it do?
// This program has "no behavior".
#[should_panic]
fn impossible_align() {
    let align = 2u128.pow(65);
    let align = Align::from_bytes(align).unwrap();

    let pty = ptype(<u8>::get_type(), align);

    let locals = [ pty ];

    let b0 = block!(
        storage_live(0),
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_stop(p); // will panic!
}
