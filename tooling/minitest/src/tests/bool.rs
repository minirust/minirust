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
    assert_stop::<BasicMem>(program);
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
    assert_stop::<BasicMem>(program);
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
    assert_stop::<BasicMem>(program);
}

/// Test that BoolBinOp::BitAnd works
#[test]
fn bit_and_bool_works() {
    let locals = [];
    let unreach_block = 5;
    let blocks = [
        // if false go to next block
        block!(if_(bool_and(const_bool(false), const_bool(false)), unreach_block, 1)),
        block!(if_(bool_and(const_bool(false), const_bool(true)), unreach_block, 2)),
        block!(if_(bool_and(const_bool(true), const_bool(false)), unreach_block, 3)),
        // if true go to next block
        block!(if_(bool_and(const_bool(true), const_bool(true)), 4, unreach_block)),
        block!(exit()),
        block!(unreachable()),
    ];
    let prog = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop::<BasicMem>(prog);
}

/// Test that BoolBinOp::BitOr works
#[test]
fn bool_or_works() {
    let locals = [];
    let unreach_block = 5;
    let blocks = [
        block!(if_(bool_or(const_bool(false), const_bool(false)), unreach_block, 1)),
        block!(if_(bool_or(const_bool(false), const_bool(true)), 2, unreach_block)),
        block!(if_(bool_or(const_bool(true), const_bool(false)), 3, unreach_block)),
        block!(if_(bool_or(const_bool(true), const_bool(true)), 4, unreach_block)),
        block!(exit()),
        block!(unreachable()),
    ];
    let prog = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop::<BasicMem>(prog);
}

/// Test that BoolBinOp::BitXor works
#[test]
fn bool_xor_works() {
    let locals = [];
    let unreach_block = 5;
    let blocks = [
        block!(if_(bool_xor(const_bool(false), const_bool(false)), unreach_block, 1)),
        block!(if_(bool_xor(const_bool(false), const_bool(true)), 2, unreach_block)),
        block!(if_(bool_xor(const_bool(true), const_bool(false)), 3, unreach_block)),
        block!(if_(bool_xor(const_bool(true), const_bool(true)), unreach_block, 4)),
        block!(exit()),
        block!(unreachable()),
    ];
    let prog = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop::<BasicMem>(prog);
}
