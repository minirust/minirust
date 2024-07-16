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

#[test]
fn pointer_rel() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let local1 = f.declare_local::<i32>();
    let local2 = f.declare_local::<i32>();
    f.storage_live(local1);
    f.storage_live(local2);
    let addr1 = addr_of(local1, <*const i32>::get_type());
    let addr2 = addr_of(local2, <*const i32>::get_type());
    f.assume(eq(addr1, addr1));
    f.assume(ne(addr1, addr2));
    f.assume(bool_or(lt(addr1, addr2), gt(addr1, addr2)));
    f.assume(ne(cmp(addr1, addr2), const_int(0i8)));
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}
