use crate::*;

/// Ill-formed: Downcasting to an out-of-bounds variant.
#[test]
fn out_of_bounds_downcast() {
    let u8_t = int_ty(Signedness::Unsigned, size(1));
    let enum_ty = enum_ty(&[enum_variant(u8_t, &[])], Discriminator::Known(0.into()), 1, size(1), align(1));
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
    let enum_ty = enum_ty(&[enum_variant(u8_t, &[])], Discriminator::Known(0.into()), 1, size(1), align(1));
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
    let variant1 = enum_variant(tuple_ty(&[(size(1), u8_t)], size(4), align(2)), &[(size(2), 0u8)]);
    let u16_t = int_ty(Signedness::Unsigned, size(2));
    let variant2 = enum_variant(tuple_ty(&[(size(0), u16_t)], size(4), align(2)), &[(size(2), 1u8)]);
    let discriminator = Discriminator::Branch {
        offset: size(2),
        fallback: GcCow::new(Discriminator::Invalid),
        children: [(0, Discriminator::Known(0.into())), (1, Discriminator::Known(1.into()))].into_iter().collect()
    };
    let enum_ty = enum_ty(&[variant1, variant2], discriminator, 1, size(4), align(2));

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
    let variant1 = enum_variant(tuple_ty(&[(size(1), u8_t)], size(4), align(2)), &[(size(2), 0)]);
    let u16_t = int_ty(Signedness::Unsigned, size(2));
    let variant2 = enum_variant(tuple_ty(&[(size(0), u16_t)], size(4), align(2)), &[(size(2), 1)]);
    let discriminator = Discriminator::Branch {
        offset: size(2),
        fallback: GcCow::new(Discriminator::Invalid),
        children: [(0, Discriminator::Known(0.into())), (1, Discriminator::Known(1.into()))].into_iter().collect()
    };
    let enum_ty = enum_ty(&[variant1, variant2], discriminator, 1, size(4), align(2));

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
