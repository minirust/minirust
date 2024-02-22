use crate::*;

const U8_INTTYPE: IntType = IntType { signed: Signedness::Unsigned, size: Size::from_bytes_const(1) };

/// Ill-formed: Downcasting to an out-of-bounds variant.
#[test]
fn out_of_bounds_downcast() {
    let u8_t = int_ty(Signedness::Unsigned, size(1));
    let enum_ty = enum_ty::<u8>(&[(0, enum_variant(u8_t, &[]))], discriminator_known(0), size(1), align(1));
    let locals = &[enum_ty, u8_t];
    let stmts = &[
        storage_live(0),
        storage_live(1),
        assign(local(1), load(downcast(local(0), 1))), // ill-formed here, variant 1 doesn't exist
    ];
    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}

/// Works: Both assigning to and from a downcast.
#[test]
fn valid_downcast() {
    let u8_t = int_ty(Signedness::Unsigned, size(1));
    let enum_ty = enum_ty::<u8>(&[(0.into(), enum_variant(u8_t, &[]))], discriminator_known(0), size(1), align(1));
    let locals = &[enum_ty, u8_t];
    let stmts = &[
        storage_live(0),
        storage_live(1),
        assign(downcast(local(0), 0), const_int(42u8)),
        assign(local(1), load(downcast(local(0), 0))),
    ];
    let prog = small_program(locals, stmts);
    assert_stop(prog);
}


/// UB: Assigning to first byte of variant 0 doesn't init both data bytes of variant 1.
#[test]
fn downcasts_give_different_place() {
    // setup enum where the first two bytes are data (u8 / u16) and the third byte is the tag.
    let u8_t = int_ty(Signedness::Unsigned, size(1));
    let variant1 = enum_variant(tuple_ty(&[(offset(1), u8_t)], size(4), align(2)), &[(offset(2), (U8_INTTYPE, 0.into()))]);
    let u16_t = int_ty(Signedness::Unsigned, size(2));
    let variant2 = enum_variant(tuple_ty(&[(offset(0), u16_t)], size(4), align(2)), &[(offset(2), (U8_INTTYPE, 1.into()))]);
    let discriminator = discriminator_branch::<u8>(
        offset(2),
        discriminator_invalid(),
        &[((0, 1), discriminator_known(0)), ((1, 2), discriminator_known(1))]
    );
    let enum_ty = enum_ty::<u8>(&[(0.into(), variant1), (1.into(), variant2)], discriminator, size(4), align(2));

    let locals = &[enum_ty, u16_t];
    let stmts = &[
        storage_live(0),
        storage_live(1),
        assign(field(downcast(local(0), 0), 0), const_int(42u8)),
        assign(local(1), load(field(downcast(local(0), 1), 0))), // UB here, only the lower byte is initialized
    ];
    let prog = small_program(locals, stmts);
    assert_ub(prog, "load at type Int(IntType { signed: Unsigned, size: Size(2 bytes) }) but the data in memory violates the validity invariant");
}

/// Works: Assigning to both bytes of variant 1 allows reads from variant 0.
#[test]
fn downcasts_give_different_place2() {
    // setup enum where the first two bytes are data (u8 / u16) and the third byte is the tag.
    let u8_t = int_ty(Signedness::Unsigned, size(1));
    let variant1 = enum_variant(tuple_ty(&[(offset(1), u8_t)], size(4), align(2)), &[(offset(2), (U8_INTTYPE, 0.into()))]);
    let u16_t = int_ty(Signedness::Unsigned, size(2));
    let variant2 = enum_variant(tuple_ty(&[(offset(0), u16_t)], size(4), align(2)), &[(offset(2), (U8_INTTYPE, 1.into()))]);
    let discriminator = discriminator_branch::<u8>(
        offset(2),
        discriminator_invalid(),
        &[((0, 1), discriminator_known(0)), ((1, 2), discriminator_known(1))]
    );
    let enum_ty = enum_ty::<u8>(&[(0.into(), variant1), (1.into(), variant2)], discriminator, size(4), align(2));

    let locals = &[enum_ty, u8_t];
    let stmts = &[
        storage_live(0),
        storage_live(1),
        assign(field(downcast(local(0), 1), 0), const_int(42u16)),
        assign(local(1), load(field(downcast(local(0), 0), 0))),
    ];
    let prog = small_program(locals, stmts);
    assert_stop(prog);
}
