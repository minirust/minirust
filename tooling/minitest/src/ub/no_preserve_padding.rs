use crate::*;

// see https://github.com/rust-lang/miri/issues/845
#[test]
fn no_preserve_padding() {
    // type Pair = (u8, u16);
    // union Union { f0: Pair, f1: u32 }
    //
    // let _0: Union;
    // let _1: Pair;
    // let _2: *const u8;
    // let _3: u8;
    //
    // _0.f1 = 0;
    // _1 = _0.f0;
    // _2 = &raw _1;
    // _2 = load(_2).offset(1)
    // _3 = *_2;

    let pair_ty = tuple_ty(&[
            (size(0), u8::get_type()),
            (size(2), u16::get_type())
        ], size(4));
    let pair_pty = ptype(pair_ty, align(2));

    let union_ty = union_ty(&[
            (size(0), pair_ty),
            (size(0), u32::get_type()),
        ], size(4));
    let union_pty = ptype(union_ty, align(4));

    let locals = vec![
        union_pty,
        pair_pty,
        <*const u8>::get_ptype(),
        <u8>::get_ptype(),
    ];

    let stmts = vec![
        storage_live(0),
        storage_live(1),
        storage_live(2),
        storage_live(3),
        assign(
            field(local(0), 1),
            const_int::<u32>(0)
        ),
        assign(
            local(1),
            load(field(local(0), 0))
        ),
        assign(
            local(2),
            addr_of(
                local(1),
                <*const u8>::get_type(),
            ),
        ),
        assign(
            local(2),
            ptr_offset(
                load(local(2)),
                const_int::<u32>(1),
                InBounds::Yes,
            )
        ),
        assign(
            local(3),
            load(deref(load(local(2)), <u8>::get_ptype())),
        ),
    ];

    let p = small_program(&locals, &stmts);
    dump_program(p);
    assert_ub(p, "load at type PlaceType { ty: Int(IntType { signed: Unsigned, size: Size { raw: Int(Small(1)) } }), align: Align { raw: Int(Small(1)) } } but the data in memory violates the validity invariant");
}
