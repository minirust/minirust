use crate::*;

const U8_INTTYPE: IntType = IntType { signed: Signedness::Unsigned, size: Size::from_bytes_const(1) };

/// It is ill-formed to write an invalid discriminant.
#[test]
fn ill_formed_invalid_discriminant_set() {
    let enum_ty = enum_ty::<u8>(&[], discriminator_invalid(), size(0), align(1));
    let locals = [enum_ty];
    let stmts = [
        storage_live(0),
        set_discriminant(local(0), 0), // ill-formed here
    ];
    let program = small_program(&locals, &stmts);
    assert_ill_formed(program);
}

/// Tests that both `get_discriminant` and `set_discriminant` generally work.
#[test]
fn discriminant_get_and_set_work() {
    // single-variant enum without data and the tag 4 for the only variant
    let enum_ty = enum_ty::<u8>(
        &[(0, enum_variant(tuple_ty(&[], size(1), align(1)), &[(offset(0), (U8_INTTYPE, 4.into()))]))],
        discriminator_branch::<u8>(
            offset(0),
            discriminator_invalid(),
            &[(4, discriminator_known(0))]
        ),
        size(1),
        align(1)
    );
    let locals = [enum_ty];

    // check that discriminant matches whats written, go to unreachable if not
    let blocks = [
        block!(
            storage_live(0),
            set_discriminant(local(0), 0),
            if_(eq(get_discriminant(local(0)), const_int(0u8)), 1, 2)
        ),
        block!(exit()),
        block!(unreachable()),
    ];
    let function = function(Ret::No, 0, &locals, &blocks);
    let program = program(&[function]);
    assert_stop(program);
}

/// Tests that `set_discriminant` actually sets the right values for all variants.
#[test]
fn discriminant_setting_right_value() {
    // multi-variant enum without data and the tags 4 and 2.
    let enum_ty = enum_ty::<u8>(
        &[
            (0, enum_variant(tuple_ty(&[], size(1), align(1)), &[(offset(0), (U8_INTTYPE, 4.into()))])),
            (1, enum_variant(tuple_ty(&[], size(1), align(1)), &[(offset(0), (U8_INTTYPE, 2.into()))])),
        ],
        discriminator_branch::<u8>(
            offset(0),
            discriminator_invalid(),
            &[(4, discriminator_known(0)), (2, discriminator_known(1))]
        ),
        size(1),
        align(1)
    );
    let locals = [union_ty(&[(offset(0), enum_ty), (offset(0), int_ty(Signedness::Unsigned, size(1)))], size(1), align(1))];

    // check that discriminant matches whats written, go to unreachable if not
    let blocks = [
        block!(
            storage_live(0),
            set_discriminant(field(local(0), 0), 0),
            if_(eq(load(field(local(0), 1)), const_int(4u8)), 1, 3)
        ),
        block!(
            set_discriminant(field(local(0), 0), 1),
            if_(eq(load(field(local(0), 1)), const_int(2u8)), 2, 3)
        ),
        block!(exit()),
        block!(unreachable()),
    ];
    let function = function(Ret::No, 0, &locals, &blocks);
    let program = program(&[function]);
    assert_stop(program);
}

/// Tests the integrity of the enum data after set_discriminant.
#[test]
fn discriminant_leaves_data_alone() {
    let u8_t = int_ty(Signedness::Unsigned, size(1));
    let u16_t = int_ty(Signedness::Unsigned, size(2));

    // single-variant enum with layout <u8 data, u8 tag, u16 data> and tag 1
    let enum_ty = enum_ty::<u8>(
        &[(0, enum_variant(tuple_ty(&[(offset(0), u8_t), (offset(2), u16_t)], size(4), align(2)), &[(offset(1), (U8_INTTYPE, 1.into()))]))],
        discriminator_branch::<u8>(
            offset(1),
            discriminator_invalid(),
            &[(1, discriminator_known(0))]
        ),
        size(4), align(2)
    );
    // the only local is a union of the enum and all its field seperately
    let locals = [union_ty(&[(offset(0), enum_ty), (offset(0), u8_t), (offset(1), u8_t), (offset(2), u16_t)], size(4), align(2))];

    let blocks = [
        block!(
            // setup enum
            storage_live(0),
            assign(field(downcast(field(local(0), 0), 0), 0), const_int(12u8)),
            assign(field(downcast(field(local(0), 0), 0), 1), const_int(9834u16)),
            set_discriminant(field(local(0), 0), 0),
            // now let the checks begin
            if_(eq(load(field(local(0), 1)), const_int(12u8)), 1, 4)
        ),
        block!(if_(eq(load(field(local(0), 2)), const_int(1u8)), 2, 4)),
        block!(if_(eq(load(field(local(0), 3)), const_int(9834u16)), 3, 4)),
        block!(exit()),
        block!(unreachable())
    ];
    let function = function(Ret::No, 0, &locals, &blocks);
    let program = program(&[function]);
    assert_stop(program);
}

/// Tests that set_discriminant does not init the data byte.
#[test]
fn ub_discriminant_does_not_init() {
    // single variant enum with layout (u8 data, u8 tag) and tag 1
    let enum_ty = enum_ty::<u8>(
        &[(0, enum_variant(tuple_ty(&[(offset(0), int_ty(Signedness::Unsigned, size(1)))], size(2), align(1)), &[(offset(1), (U8_INTTYPE, 1.into()))]))],
        discriminator_branch::<u8>(
            offset(1),
            discriminator_invalid(),
            &[(1, discriminator_known(0))]
        ),
        size(2), align(1)
    );
    let locals = [enum_ty];
    let blocks = [
        block!(
            storage_live(0),
            set_discriminant(local(0), 0),
            if_(eq(load(field(downcast(local(0), 0), 0)), const_int(0u8)), 1, 2) // ub here as the field isn't initialized
        ),
        block!(exit()),
        block!(unreachable()),
    ];
    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_ub(program, "load at type Int(IntType { signed: Unsigned, size: Size(1 bytes) }) but the data in memory violates the validity invariant");
}

/// Tests that reading from a discriminant that wasn't initialized is UB.
#[test]
fn ub_cannot_read_uninit_discriminant() {
    // single variant enum with layout (u8 data, u8 tag) and tag 1
    let enum_ty = enum_ty::<u8>(
        &[(0, enum_variant(tuple_ty(&[(offset(0), int_ty(Signedness::Unsigned, size(1)))], size(2), align(1)), &[(offset(1), (U8_INTTYPE, 1.into()))]))],
        discriminator_branch::<u8>(
            offset(1),
            discriminator_invalid(),
            &[(1, discriminator_known(0))]
        ),
        size(2), align(1)
    );
    let locals = [enum_ty];
    let blocks = [
        block!(
            storage_live(0),
            assign(field(downcast(local(0), 0), 0), const_int(12u8)),
            if_(eq(const_int(42u8), get_discriminant(local(0))), 1, 2) // ub here as the discriminant isn't initialized
        ),
        block!(exit()),
        block!(unreachable()),
    ];
    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_ub(program, "ValueExpr::GetDiscriminant encountered invalid discriminant.");
}

/// Tests that reading from an invalid discriminant is UB.
#[test]
fn ub_cannot_read_invalid_discriminant() {
    let u8_t = int_ty(Signedness::Unsigned, size(1));
    // single variant enum without data and tag 1
    let enum_ty = enum_ty::<u8>(
        &[(0, enum_variant(tuple_ty(&[], size(1), align(1)), &[(offset(0), (U8_INTTYPE, 1.into()))]))],
        discriminator_branch::<u8>(
            offset(0),
            discriminator_invalid(),
            &[(1, discriminator_known(0))]
        ),
        size(1), align(1)
    );
    let locals = [union_ty(&[(offset(0), enum_ty), (offset(0), u8_t)], size(1), align(1))];
    let blocks = [
        block!(
            storage_live(0),
            assign(field(local(0), 1), const_int(12u8)),
            if_(eq(const_int(12u8), get_discriminant(field(local(0), 0))), 1, 2) // ub here as the discriminant isn't valid
        ),
        block!(exit()),
        block!(unreachable()),
    ];
    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_ub(program, "ValueExpr::GetDiscriminant encountered invalid discriminant.");
}

/// Reading discriminant from mis-aligned enum (ptr) is UB.
#[test]
fn ub_get_discriminant_on_misaligned_enum() {
    let enum_t = enum_ty::<u8>(&[(0, enum_variant(<u16>::get_type(), &[]))], discriminator_known(0), size(2), align(2));
    let raw_ptr_t = <*const [u16;2]>::get_type();
    let locals = [<[u16;2]>::get_type(), <u8>::get_type()];
    let stmts = [
        storage_live(0),
        storage_live(1),
        assign(local(0), array(&[const_int(0u16), const_int(0u16)], <u16>::get_type())),
        assign(local(1), get_discriminant(deref(ptr_offset(addr_of(local(0), raw_ptr_t), const_int(1u8), InBounds::Yes), enum_t))),
    ];
    let prog = small_program(&locals, &stmts);
    assert_ub(prog, "Getting the discriminant of a place based on a misaligned pointer.");
}

/// Setting discriminant of mis-aligned enum (ptr) is UB.
#[test]
fn ub_set_discriminant_on_misaligned_enum() {
    let enum_t = enum_ty::<u8>(&[(0, enum_variant(<u16>::get_type(), &[]))], discriminator_known(0), size(2), align(2));
    let raw_ptr_t = <*const [u16;2]>::get_type();
    let locals = [<[u16;2]>::get_type()];
    let stmts = [
        storage_live(0),
        assign(local(0), array(&[const_int(0u16), const_int(0u16)], <u16>::get_type())),
        set_discriminant(deref(ptr_offset(addr_of(local(0), raw_ptr_t), const_int(1u8), InBounds::Yes), enum_t), 0),
    ];
    let prog = small_program(&locals, &stmts);
    assert_ub(prog, "Setting the discriminant of a place based on a misaligned pointer");
}

/// Ensures that the behaviour of an `Option<NonZeroU8>` of Rust is possible in MiniRust.
#[test]
fn space_optimized_enum_works() {
    let u8_t = int_ty(Signedness::Unsigned, size(1));
    // a space-optimized version of `Option<NonZeroU8>` based on an actual u8
    let enum_ty = enum_ty::<u8>(
        &[
            (0, enum_variant(u8_t, &[])),
            (1, enum_variant(tuple_ty(&[], size(1), align(1)), &[(offset(0), (U8_INTTYPE, 0.into()))])),
        ],
        discriminator_branch::<u8>(
            offset(0),
            discriminator_known(0),
            &[(0, discriminator_known(1))]
        ),
        size(1), align(1)
    );
    let locals = [union_ty(&[(offset(0), enum_ty), (offset(0), u8_t)], size(1), align(1))];
    let blocks = [
        block!( // write variant 1 and see that the byte is now 0
            storage_live(0),
            set_discriminant(field(local(0), 0), 1),
            if_(eq(load(field(local(0), 1)), const_int(0u8)), 1, 3),
        ),
        block!( // write variant 0 with value 42 and see that the byte is now 42
            assign(downcast(field(local(0), 0), 0), const_int(42u8)),
            set_discriminant(field(local(0), 0), 0),
            if_(eq(load(field(local(0), 1)), const_int(42u8)), 2, 3),
        ),
        block!(exit()),
        block!(unreachable()),
    ];
    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop(program);
}
