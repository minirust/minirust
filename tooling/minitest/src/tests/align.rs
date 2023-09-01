use crate::*;

#[test]
fn manual_align() {
    let locals = &[
        <[u8; 64]>::get_type(),
        <usize>::get_type()
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
                    const_int(8usize),
                    mul::<usize>(
                        div::<usize>(
                            load(local(1)),
                            const_int(8usize)
                        ),
                        const_int(8usize)
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
                <u64>::get_type()
            ),
            const_int(42u64)
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

    let ty = tuple_ty(&[], size(0), align);

    let locals = [ ty ];

    let b0 = block!(
        storage_live(0),
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_stop(p); // will panic!
}

#[test]
fn load_place_misaligned() {
    let union_ty = union_ty(&[
            (size(0), <usize>::get_type()),
            (size(0), <*const [i32; 0]>::get_type()),
        ], size(8), align(8));

    let locals = [ union_ty, <[i32; 0]>::get_type(), ];

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(
            field(local(0), 0),
            const_int(1usize) // nullptr + 1
        ),
        assign(
            local(1),
            load(deref(load(field(local(0), 1)), <[i32; 0]>::get_type()))
        ),
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "loading from a place based on a misaligned pointer");
}

#[test]
fn store_place_misaligned() {
    let union_ty = union_ty(&[
            (size(0), <usize>::get_type()),
            (size(0), <*const [i32; 0]>::get_type()),
        ], size(8), align(8));

    let locals = [ union_ty, <[i32; 0]>::get_type(), ];

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(
            field(local(0), 0),
            const_int(1usize) // nullptr + 1
        ),
        assign(
            deref(load(field(local(0), 1)), <[i32; 0]>::get_type()),
            load(local(1)),
        ),
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "storing to a place based on a misaligned pointer");
}

#[test]
fn deref_misaligned_ref() {
    let locals = [ <*const i32>::get_type(), <*const u8>::get_type() ];
    let b0 = block!(
        storage_live(0),
        allocate(
            const_int(4usize),
            const_int(4usize),
            local(0),
            1,
        )
    );
    let u8ptr = ptr_to_ptr(load(local(0)), <*const u8>::get_type());
    // make the pointer definitely not 2-aligned
    let nonaligned = ptr_offset(u8ptr, const_int(1usize), InBounds::Yes);
    let u16ptr = ptr_to_ptr(nonaligned, <*const u16>::get_type());
    let u16ref = transmute(u16ptr, <&u16>::get_type());
    let b1 = block!(
        storage_live(1),
        assign(
            local(1),
            // We deref to type `u8`, but the alignment of the reference matters!
            addr_of(deref(u16ref, u8::get_type()), <*const u8>::get_type()),
        ),
        exit(),
    );
    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "transmuted value is not valid at new type");
}

#[test]
fn deref_overaligned() {
    let locals = [ <i32>::get_type(), <*const i32>::get_type(), <u32>::get_type() ];
    let b0 = block!(
        storage_live(0),
        assign(
            local(0),
            const_int(0i32),
        ),
        storage_live(1),
        assign(
            local(1),
            addr_of(local(1), <*const i32>::get_type()),
        ),
        goto(1),
    );
    let u8ptr = ptr_to_ptr(load(local(1)), <*const u8>::get_type());
    let b1 = block!(
        storage_live(2),
        assign(
            local(2),
            // We deref a `*const u8` to a `u32` and load that but it's fine since it actually is u32-aligned.
            load(deref(u8ptr, u32::get_type())),
        ),
        exit(),
    );
    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_stop(p);
}

#[test]
fn addr_of_misaligned_ref() {
    let locals = [ <i32>::get_type(), <*const i32>::get_type(), <&u16>::get_type() ];
    let b0 = block!(
        storage_live(0),
        assign(
            local(0),
            const_int(0i32),
        ),
        storage_live(1),
        assign(
            local(1),
            addr_of(local(1), <*const i32>::get_type()),
        ),
        goto(1),
    );
    let u8ptr = ptr_to_ptr(load(local(1)), <*const u8>::get_type());
    // make the pointer definitely not 2-aligned
    let nonaligned = ptr_offset(u8ptr, const_int(1usize), InBounds::Yes);
    let u16ptr = ptr_to_ptr(nonaligned, <*const u16>::get_type());
    let b1 = block!(
        storage_live(2),
        assign(
            local(2),
            // We deref to `u8` but the type of the reference matters!
            addr_of(deref(u16ptr, <u8>::get_type()), <&u16>::get_type()),
        ),
        exit(),
    );
    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "taking the address of an invalid (null, misaligned, or uninhabited) place");
}

/// Same test as above, but with a raw pointer it's fine.
#[test]
fn addr_of_misaligned_ptr() {
    let locals = [ <i32>::get_type(), <*const i32>::get_type(), <*const u16>::get_type() ];
    let b0 = block!(
        storage_live(0),
        assign(
            local(0),
            const_int(0i32),
        ),
        storage_live(1),
        assign(
            local(1),
            addr_of(local(1), <*const i32>::get_type()),
        ),
        goto(1),
    );
    let u8ptr = ptr_to_ptr(load(local(1)), <*const u8>::get_type());
    // make the pointer definitely not 2-aligned
    let nonaligned = ptr_offset(u8ptr, const_int(1usize), InBounds::Yes);
    let u16ptr = ptr_to_ptr(nonaligned, <*const u16>::get_type());
    let b1 = block!(
        storage_live(2),
        assign(
            local(2),
            // This ptr is not aligned to `u16`, and then we even addr_of to `u32` here which
            // requires even more alignment, but it's all raw so it's fine.
            addr_of(deref(u16ptr, <u16>::get_type()), <*const u32>::get_type()),
        ),
        exit(),
    );
    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_stop(p);
}
