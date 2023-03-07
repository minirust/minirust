use crate::*;

#[test]
#[should_panic]
fn impossible_align() { // TODO this should not actually panic!
    let align = 2u128.pow(65);
    let align = Align::from_bytes(align).unwrap();

    let pty = ptype(<u8>::get_type(), align);

    let locals = [ pty ];

    let b0 = block2(&[
        &live(0),
        &exit()
    ]);

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    dump_program(&p);
    assert_stop(p); // will panic!
}
