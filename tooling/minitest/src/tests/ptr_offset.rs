use crate::*;

#[test]
fn ptr_offset_success() {
    let locals = &[<i32>::get_type(), <*const i32>::get_type()];

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<i32>(42),),
        assign(local(1), addr_of(local(0), <*const i32>::get_type())),
        assign(local(1), ptr_offset(load(local(1)), const_int::<usize>(4), InBounds::Yes,)),
        exit()
    );

    let f = function(Ret::No, 0, locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_stop::<BasicMem>(p);
}

#[test]
fn ptr_offset_inbounds() {
    let locals = &[<i32>::get_type(), <*const i32>::get_type()];

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<i32>(42),),
        assign(local(1), addr_of(local(0), <*const i32>::get_type())),
        assign(
            local(1),
            ptr_offset(load(local(1)), const_int::<usize>(usize::MAX), InBounds::Yes,)
        ),
        exit()
    );

    let f = function(Ret::No, 0, locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub::<BasicMem>(p, "dereferencing pointer outside the bounds of its allocation");
}

#[test]
fn ptr_offset_no_inbounds() {
    let locals = &[<i32>::get_type(), <*const i32>::get_type()];

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<i32>(42),),
        assign(local(1), addr_of(local(0), <*const i32>::get_type())),
        assign(
            local(1),
            ptr_offset(
                load(local(1)),
                const_int::<usize>(usize::MAX), // this huge offset is out of range, but InBounds::No cannot fail.
                InBounds::No,
            )
        ),
        exit()
    );

    let f = function(Ret::No, 0, locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_stop::<BasicMem>(p);
}

#[test]
fn ptr_offset_out_of_bounds() {
    let locals = &[<i32>::get_type(), <*const i32>::get_type()];

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<i32>(42),),
        assign(local(1), addr_of(local(0), <*const i32>::get_type())),
        assign(
            local(1),
            ptr_offset(
                load(local(1)),
                const_int::<usize>(5), // an offset of 5 is too large for an allocation of 4 bytes!
                InBounds::Yes,
            )
        ),
        exit()
    );

    let f = function(Ret::No, 0, locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub::<BasicMem>(p, "dereferencing pointer outside the bounds of its allocation");
}

#[test]
fn invalid_offset() {
    let union_ty = union_ty(
        &[(size(0), <*const i32>::get_type()), (size(0), <usize>::get_type())],
        size(8),
        align(8),
    );
    let locals = &[<[i32; 2]>::get_type(), union_ty];

    let stmts = &[
        storage_live(0),
        storage_live(1),
        assign(local(0), array(&[const_int::<i32>(42), const_int::<i32>(24)], <i32>::get_type())),
        assign(
            field(local(1), 0),
            addr_of(index(local(0), const_int::<usize>(0)), <*const i32>::get_type()),
        ),
        assign(
            // strips provenance!
            field(local(1), 1),
            load(field(local(1), 1)),
        ),
        assign(
            field(local(1), 0),
            ptr_offset(load(field(local(1), 0)), const_int::<usize>(4), InBounds::Yes),
        ),
    ];

    let p = small_program(locals, stmts);
    dump_program(p);
    assert_ub::<BasicMem>(p, "dereferencing pointer without provenance");
}
