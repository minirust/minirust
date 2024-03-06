use crate::*;

/// Tests that false to int results in 0.
#[test]
fn false_to_int_works() {
    let locals = [];
    let blocks = [
        block!(switch_int(bool_to_int::<u8>(const_bool(false)), &[(0u8, 1)], 2)),
        block!(exit()),
        block!(unreachable()),
    ];

    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop(program);
}

/// Tests that true to int results in 1.
#[test]
fn true_to_int_works() {
    let locals = [];
    let blocks = [
        block!(switch_int(bool_to_int::<u8>(const_bool(true)), &[(1u8, 1)], 2)),
        block!(exit()),
        block!(unreachable()),
    ];

    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop(program);
}

/// Tests that boolean negation works.
#[test]
fn not_works_both_ways() {
    let locals = [];
    let blocks = [
        block!(if_(not(const_bool(false)), 1, 3)), // go to next block if !false
        block!(if_(not(const_bool(true)), 3, 2)),  // go to unreachable if !true
        block!(exit()),
        block!(unreachable()),
    ];

    let program = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop(program);
}

/// Tests that boolean not requires a boolean operand
#[test]
fn boolean_not_requires_boolean_op() {
    let locals = &[<bool>::get_type()];
    let statements = &[storage_live(0), assign(local(0), not(const_int(0u8))), storage_dead(0)];
    let program = small_program(locals, statements);
    assert_ill_formed(program);
}

/// Tests that bool2int requires a boolean operand
#[test]
fn bool2int_requires_boolean_op() {
    let locals = &[<u8>::get_type()];
    let statements =
        &[storage_live(0), assign(local(0), bool_to_int::<u8>(const_int(0u8))), storage_dead(0)];
    let program = small_program(locals, statements);
    assert_ill_formed(program);
}
