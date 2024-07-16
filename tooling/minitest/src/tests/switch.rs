use crate::*;

/// tests that the if case can be reached.
/// Also tests that BoolToIntCast converts true to 1.
#[test]
fn if_works() {
    let locals = [];
    let blocks = [block!(if_(const_bool(true), 1, 2)), block!(exit()), block!(unreachable())];

    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop::<BasicMem>(program);
}

/// tests that the else case can be reached.
/// Also tests that BoolToIntCast converts false to 0.
#[test]
fn else_works() {
    let locals = [];
    let blocks = [block!(if_(const_bool(false), 1, 2)), block!(unreachable()), block!(exit())];

    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop::<BasicMem>(program);
}

/// tests that an integer switch that switches on a boolean is ill-formed.
#[test]
fn boolean_switch_is_ill_formed() {
    let locals = [];
    let blocks = [
        block!(switch_int(const_bool(false), &[(0u8, 1)], 1)), // ill-formed here at const_bool as switch_int creates int cases
        block!(unreachable()),
    ];

    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_ill_formed::<BasicMem>(program, "Terminator::Switch: switch is not Int");
}

/// tests that switch_int can access an arbitrary case and the fallback case.
#[test]
fn switch_int_works() {
    let locals = [];
    let blocks = [
        block!(switch_int(const_int(1u8), &[(0u8, 3), (1u8, 1)], 3)),
        block!(switch_int(const_int(12u8), &[(0u8, 3), (1u8, 3)], 2)),
        block!(exit()),
        block!(unreachable()),
    ];

    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop::<BasicMem>(program);
}

const U8_INTTYPE: IntType =
    IntType { signed: Signedness::Unsigned, size: Size::from_bytes_const(1) };

/// tests that switching on a enum discriminant is possible
#[test]
fn switch_enum_works() {
    let enum_ty = enum_ty::<u8>(
        &[
            (
                0,
                enum_variant(
                    tuple_ty(&[], size(1), align(1)),
                    &[(offset(0), (U8_INTTYPE, 4.into()))],
                ),
            ),
            (
                1,
                enum_variant(
                    tuple_ty(&[], size(1), align(1)),
                    &[(offset(0), (U8_INTTYPE, 2.into()))],
                ),
            ),
        ],
        discriminator_branch::<u8>(
            offset(0),
            discriminator_invalid(),
            &[((2, 3), discriminator_known(1)), ((4, 5), discriminator_known(0))],
        ),
        size(1),
        align(1),
    );
    let locals = [enum_ty];
    let blocks = [
        block!(
            storage_live(0),
            set_discriminant(local(0), 0),
            switch_int(get_discriminant(local(0)), &[(0u8, 1), (1u8, 2)], 2)
        ),
        block!(exit()),
        block!(unreachable()),
    ];

    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop::<BasicMem>(program);
}
