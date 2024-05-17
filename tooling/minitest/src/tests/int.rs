use crate::*;

/// Test that IntBinOp::BitAnd works for ints
#[test]
fn bit_and_int_works() {
    let locals = [];
    let unreach_block = 5;
    let bit_and = |x, y| bit_and(const_int::<i32>(x), const_int::<i32>(y));

    let blocks = [
        block!(if_(eq(bit_and(171, 62), const_int::<i32>(42)), 1, unreach_block)),
        block!(if_(eq(bit_and(171, -214), const_int::<i32>(42)), 2, unreach_block)),
        block!(if_(eq(bit_and(-2645, 62), const_int::<i32>(42)), 3, unreach_block)),
        block!(if_(eq(bit_and(-41, -10), const_int::<i32>(-42)), 4, unreach_block)),
        block!(exit()),
        block!(unreachable()),
    ];

    let prog = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop(prog);
}

// Test that IntBinOp::BitAnd fails with non-int/non-bool
#[test]
fn bit_and_requires_int() {
    let locals = [<i32>::get_type()];
    let const_arr = array(&[const_int::<u8>(0); 3], <u8>::get_type());
    let b0 = block!(
        storage_live(0),
        assign(local(0), bit_and(const_arr, const_arr)),
        storage_dead(0),
        exit(),
    );
    let prog = program(&[function(Ret::No, 0, &locals, &[b0])]);
    assert_ill_formed(prog, "BinOp::Int: invalid left type");
}

// Test that IntBinOp::BitAnd fails with bool
#[test]
fn bit_and_no_int_bool_mixing() {
    let locals = [<i32>::get_type()];
    let b0 = block!(
        storage_live(0),
        assign(local(0), bit_and(const_bool(false), const_bool(true))),
        storage_dead(0),
        exit(),
    );
    let prog = program(&[function(Ret::No, 0, &locals, &[b0])]);
    assert_ill_formed(prog, "BinOp::Int: invalid left type");
}

/// Test that IntBinOp::BitOr works for ints
#[test]
fn bit_or_int_works() {
    let locals = [];
    let unreach_block = 5;
    let bit_or = |x, y| bit_or(const_int::<i32>(x), const_int::<i32>(y));

    let blocks = [
        block!(if_(eq(bit_or(34, 10), const_int::<i32>(42)), 1, unreach_block)),
        block!(if_(eq(bit_or(6, -46), const_int::<i32>(-42)), 2, unreach_block)),
        block!(if_(eq(bit_or(-44, 18), const_int::<i32>(-42)), 3, unreach_block)),
        block!(if_(eq(bit_or(-58, -46), const_int::<i32>(-42)), 4, unreach_block)),
        block!(exit()),
        block!(unreachable()),
    ];

    let prog = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop(prog);
}

/// Test that IntBinOp::BitXor works for ints
#[test]
fn bit_xor_int_works() {
    let locals = [];
    let unreach_block = 5;
    let bit_xor = |x, y| bit_xor(const_int::<i32>(x), const_int::<i32>(y));

    let blocks = [
        block!(if_(eq(bit_xor(14, 36), const_int::<i32>(42)), 1, unreach_block)),
        block!(if_(eq(bit_xor(6, -48), const_int::<i32>(-42)), 2, unreach_block)),
        block!(if_(eq(bit_xor(41, -1), const_int::<i32>(-42)), 3, unreach_block)),
        block!(if_(eq(bit_xor(-1, -43), const_int::<i32>(42)), 4, unreach_block)),
        block!(exit()),
        block!(unreachable()),
    ];

    let prog = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop(prog);
}

/// Test that BinUnOp::Not works for ints
#[test]
fn bit_int_not_works() {
    let locals = [];
    let unreach_block = 3;
    let not = |x| int_not(const_int::<i32>(x));

    let blocks = [
        block!(if_(eq(not(42), const_int::<i32>(-43)), 1, unreach_block)),
        block!(if_(eq(not(-43), const_int::<i32>(42)), 2, unreach_block)),
        block!(exit()),
        block!(unreachable()),
    ];

    let prog = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop(prog);
}
