use crate::*;

#[test]
fn pointer_partial_overwrite() {
    let locals = &[<i32>::get_type(), <&i32>::get_type(), <i32>::get_type()];

    let stmts = &[
        storage_live(0),
        storage_live(1),
        storage_live(2),
        assign(local(0), const_int::<i32>(42)),
        assign(local(1), addr_of(local(0), <&i32>::get_type())),
        assign(
            // this corrupts one u8 of the pointer, stripping its provenance
            deref(addr_of(local(1), <*mut u8>::get_type()), <u8>::get_type()),
            const_int::<u8>(12),
        ),
        assign(local(2), load(deref(load(local(1)), <i32>::get_type()))),
    ];

    let p = small_program(locals, stmts);
    dump_program(p);
    assert_ub::<BasicMem>(p, "dereferencing pointer without provenance");
}
