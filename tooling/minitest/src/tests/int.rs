use crate::*;

#[test]
fn arith_works() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();

    f.assume(eq(add(const_int(1u8), const_int(7u8)), const_int(8u8)));
    f.assume(eq(sub(const_int(1u8), const_int(7u8)), const_int(250u8)));
    f.assume(eq(mul(const_int(1u8), const_int(7u8)), const_int(7u8)));
    f.assume(eq(div(const_int(1u8), const_int(7u8)), const_int(0u8)));
    f.assume(eq(rem(const_int(1u8), const_int(7u8)), const_int(1u8)));

    // Division is odd around signs...
    f.assume(eq(div(const_int(7i8), const_int(3i8)), const_int(2i8)));
    f.assume(eq(rem(const_int(7i8), const_int(3i8)), const_int(1i8)));
    f.assume(eq(div(const_int(7i8), const_int(-3i8)), const_int(-2i8)));
    f.assume(eq(rem(const_int(7i8), const_int(-3i8)), const_int(1i8)));
    f.assume(eq(div(const_int(-7i8), const_int(3i8)), const_int(-2i8)));
    f.assume(eq(rem(const_int(-7i8), const_int(3i8)), const_int(-1i8)));
    f.assume(eq(div(const_int(-7i8), const_int(-3i8)), const_int(2i8)));
    f.assume(eq(rem(const_int(-7i8), const_int(-3i8)), const_int(-1i8)));

    f.exit();

    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn div_zero() {
    let locals = [<i32>::get_type()];

    let b0 = block!(
        storage_live(0),
        assign(local(0), div(const_int::<i32>(1), const_int::<i32>(0),)),
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub::<BasicMem>(p, "division by zero");
}

#[test]
fn rem_zero() {
    let locals = [<i32>::get_type()];

    let b0 = block!(
        storage_live(0),
        assign(local(0), rem(const_int::<i32>(1), const_int::<i32>(0),)),
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub::<BasicMem>(p, "modulus of remainder is zero");
}

#[test]
fn div_overflow() {
    let locals = [<i32>::get_type()];

    let b0 = block!(
        storage_live(0),
        assign(local(0), div(const_int::<i32>(i32::MIN), const_int::<i32>(-1),)),
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub::<BasicMem>(p, "overflow in division");
}

#[test]
fn rem_overflow() {
    let locals = [<i32>::get_type()];

    let b0 = block!(
        storage_live(0),
        assign(local(0), rem(const_int::<i32>(i32::MIN), const_int::<i32>(-1),)),
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub::<BasicMem>(p, "overflow in remainder");
}

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
    assert_stop::<BasicMem>(prog);
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
    assert_ill_formed::<BasicMem>(prog, "BinOp::Int: invalid left type");
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
    assert_ill_formed::<BasicMem>(prog, "BinOp::Int: invalid left type");
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
    assert_stop::<BasicMem>(prog);
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
    assert_stop::<BasicMem>(prog);
}

/// Test that BinUnOp::Not works for ints
#[test]
fn bit_int_not_works() {
    let locals = [];
    let unreach_block = 3;
    let not = |x| bit_not(const_int::<i32>(x));

    let blocks = [
        block!(if_(eq(not(42), const_int::<i32>(-43)), 1, unreach_block)),
        block!(if_(eq(not(-43), const_int::<i32>(42)), 2, unreach_block)),
        block!(exit()),
        block!(unreachable()),
    ];

    let prog = program(&[function(Ret::No, 0, &locals, &blocks)]);
    assert_stop::<BasicMem>(prog);
}

#[test]
fn shl_works() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();

    f.assume(eq(shl(const_int(1u8), const_int(7u8)), const_int(128u8)));
    f.assume(eq(shl(const_int(1u8), const_int(0u8)), const_int(1u8)));
    f.assume(eq(shl(const_int(-1i32), const_int(1i32)), const_int(-2i32)));
    f.assume(eq(shl(const_int(i32::MAX), const_int(1)), const_int(-2i32)));

    // Shl should allow for different integer types for left and right operands
    f.assume(eq(shl(const_int(1u16), const_int(7i32)), const_int(128u16)));

    // Test if shift calculates the euclidean modulo of right operand with number of bits of left
    // 8 % 8 = 0; shift by 0
    f.assume(eq(shl(const_int(1u8), const_int(8)), const_int(1u8)));
    // 9 % 8 = 1; shift by 1
    f.assume(eq(shl(const_int(1u8), const_int(9)), const_int(2u8)));
    // -1 % 8 = 7; (for solution in 0..8) shift by 7
    f.assume(eq(shl(const_int(1u8), const_int(-1)), const_int(128u8)));
    f.exit();

    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn shr_works() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();

    // Logical shr for unsigned integers
    f.assume(eq(shr(const_int(u8::MAX), const_int(7u8)), const_int(1u8)));
    f.assume(eq(shr(const_int(1u8), const_int(0)), const_int(1u8)));

    // Arithmetic shr for signed integers
    f.assume(eq(shr(const_int(-4i16), const_int(1u16)), const_int(-2i16)));
    f.assume(eq(shr(const_int(-1i32), const_int(1)), const_int(-1i32)));
    f.assume(eq(shr(const_int(1i32), const_int(1)), const_int(0i32)));
    f.assume(eq(shr(const_int(i32::MAX), const_int(1)), const_int(i32::MAX / 2)));
    f.assume(eq(shr(const_int(i32::MIN), const_int(1)), const_int(i32::MIN / 2)));

    // Shr should allow for different integer types for left and right operands
    f.assume(eq(shr(const_int(u8::MAX), const_int(7i32)), const_int(1u8)));
    f.assume(eq(shr(const_int(-4i16), const_int(1u8)), const_int(-2i16)));

    // Test if shift calculates the euclidean modulo of right operand with number of bits of left
    // 8 % 8 = 0; shift by 0
    f.assume(eq(shr(const_int(1u8), const_int(8)), const_int(1u8)));
    // 9 % 8 = 1; shift by 1
    f.assume(eq(shr(const_int(2u8), const_int(9)), const_int(1u8)));
    // -1 % 8 = 7; (for solution in 0..8) shift by 7
    f.assume(eq(shr(const_int(u8::MAX), const_int(-1)), const_int(1u8)));
    f.exit();

    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn unchecked_add_ub() {
    assert_ub_expr::<i8, BasicMem>(
        add_unchecked(const_int(i8::MAX), const_int(1_i8)),
        "overflow in unchecked add",
    );

    assert_ub_expr::<i8, BasicMem>(
        add_unchecked(const_int(i8::MIN), const_int(-1i8)),
        "overflow in unchecked add",
    );
}

#[test]
fn unchecked_add_works() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    f.assume(eq(add_unchecked(const_int(i8::MIN), const_int(i8::MAX)), const_int(-1_i8)));
    f.assume(eq(add_unchecked(const_int(0_i8), const_int(-i8::MAX)), const_int(i8::MIN + 1_i8)));
    f.assume(eq(add_unchecked(const_int(30), const_int(12)), const_int(42)));
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn unchecked_sub_ub() {
    assert_ub_expr::<i8, BasicMem>(
        sub_unchecked(const_int(i8::MIN), const_int(1_i8)),
        "overflow in unchecked sub",
    );

    assert_ub_expr::<i8, BasicMem>(
        sub_unchecked(const_int(0_i8), const_int(i8::MIN)),
        "overflow in unchecked sub",
    );
}

#[test]
fn unchecked_sub_works() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    f.assume(eq(sub_unchecked(const_int(-1_i8), const_int(i8::MAX)), const_int(i8::MIN)));
    f.assume(eq(sub_unchecked(const_int(-1_i8), const_int(i8::MIN)), const_int(i8::MAX)));
    f.assume(eq(sub_unchecked(const_int(53), const_int(11)), const_int(42)));
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn unchecked_mul_ub() {
    assert_ub_expr::<i8, BasicMem>(
        mul_unchecked(const_int(i8::MIN), const_int(-1_i8)),
        "overflow in unchecked mul",
    );
    assert_ub_expr::<i8, BasicMem>(
        mul_unchecked(const_int(56_i8), const_int(3_i8)),
        "overflow in unchecked mul",
    );
}

#[test]
fn unchecked_mul_works() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    f.assume(eq(mul_unchecked(const_int(-1_i8), const_int(i8::MAX)), const_int(i8::MIN + 1)));
    f.assume(eq(mul_unchecked(const_int(-8_i8), const_int(16_i8)), const_int(i8::MIN)));
    f.assume(eq(mul_unchecked(const_int(6), const_int(7)), const_int(42)));
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn unchecked_shl_ub() {
    // If left side is `u8` every right side not in range `0..8` causes UB
    assert_ub_expr::<u8, BasicMem>(
        shl_unchecked(const_int(1u8), const_int(8)),
        "overflow in unchecked shift",
    );
    assert_ub_expr::<u8, BasicMem>(
        shl_unchecked(const_int(1u8), const_int(9)),
        "overflow in unchecked shift",
    );
    assert_ub_expr::<u8, BasicMem>(
        shl_unchecked(const_int(1u8), const_int(-1)),
        "overflow in unchecked shift",
    );
}

#[test]
fn unchecked_shl_works() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();

    f.assume(eq(shl_unchecked(const_int(1u8), const_int(7u8)), const_int(128u8)));
    f.assume(eq(shl_unchecked(const_int(1u8), const_int(0u8)), const_int(1u8)));
    f.assume(eq(shl_unchecked(const_int(-1i32), const_int(1i32)), const_int(-2i32)));
    f.assume(eq(shl_unchecked(const_int(i32::MAX), const_int(1)), const_int(-2i32)));

    // Unchecked shl should allow for different integer types for left and right operands
    f.assume(eq(shl_unchecked(const_int(1u16), const_int(7i32)), const_int(128u16)));
    f.exit();

    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn unchecked_shr_ub() {
    // If left side is `u8` every right side not in range `0..8` causes UB
    assert_ub_expr::<u8, BasicMem>(
        shr_unchecked(const_int(1u8), const_int(8)),
        "overflow in unchecked shift",
    );
    assert_ub_expr::<u8, BasicMem>(
        shr_unchecked(const_int(2u8), const_int(9)),
        "overflow in unchecked shift",
    );
    assert_ub_expr::<u8, BasicMem>(
        shr_unchecked(const_int(2u8), const_int(9)),
        "overflow in unchecked shift",
    );
    assert_ub_expr::<u8, BasicMem>(
        shr_unchecked(const_int(u8::MAX), const_int(-1)),
        "overflow in unchecked shift",
    );
}

#[test]
fn unchecked_shr_works() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();

    // Logical shr for unsigned integers
    f.assume(eq(shr_unchecked(const_int(u8::MAX), const_int(7u8)), const_int(1u8)));
    f.assume(eq(shr_unchecked(const_int(1u8), const_int(0)), const_int(1u8)));

    // Arithmetic shr for signed integers
    f.assume(eq(shr_unchecked(const_int(-4i16), const_int(1u16)), const_int(-2i16)));
    f.assume(eq(shr_unchecked(const_int(-1i32), const_int(1)), const_int(-1i32)));
    f.assume(eq(shr_unchecked(const_int(1i32), const_int(1)), const_int(0i32)));
    f.assume(eq(shr_unchecked(const_int(i32::MAX), const_int(1)), const_int(i32::MAX / 2)));
    f.assume(eq(shr_unchecked(const_int(i32::MIN), const_int(1)), const_int(i32::MIN / 2)));

    // Unchecked shr should allow for different integer types for left and right operands
    f.assume(eq(shr_unchecked(const_int(u8::MAX), const_int(7i32)), const_int(1u8)));
    f.assume(eq(shr_unchecked(const_int(-4i16), const_int(1u8)), const_int(-2i16)));
    f.exit();

    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn cmp_works() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();

    f.assume(eq(cmp(const_int(0), const_int(-1)), const_int(1_i8)));
    f.assume(eq(cmp(const_int(-1), const_int(0)), const_int(-1_i8)));
    f.assume(eq(cmp(const_int(0), const_int(1)), const_int(-1_i8)));
    f.assume(eq(cmp(const_int(1), const_int(0)), const_int(1_i8)));
    f.assume(eq(cmp(const_int(0), const_int(0)), const_int(0_i8)));
    f.assume(eq(cmp(const_int(i128::MAX), const_int(i128::MIN)), const_int(1_i8)));
    f.exit();

    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn cmp_ill_formed_left() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let loc = f.declare_local::<i8>();
    f.storage_live(loc);
    f.assign(loc, cmp(const_bool(false), const_int(0_i32)));
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "BinOp::IntCmp: invalid left type");
}

#[test]
fn cmp_ill_formed_right() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let loc = f.declare_local::<i8>();
    f.storage_live(loc);
    f.assign(loc, cmp(const_int(0_i32), const_int(0_i8)));
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "BinOp::IntCmp: invalid right type");
}

#[test]
fn overflow_add_works() {
    let ty =
        tuple_ty(&[(Size::ZERO, i32::get_type()), (size(4), bool::get_type())], size(8), align(4));
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let loc = f.declare_local_with_ty(ty);
    f.storage_live(loc);
    let mut test = |l: i32, r: i32, result: (i32, bool)| {
        f.assign(loc, overflow_add(const_int(l), const_int(r)));
        f.assume(eq(load(field(loc, 0)), const_int(result.0)));
        f.assume(not(bool_xor(load(field(loc, 1)), const_bool(result.1))));
    };
    // test the non-overflowing addition
    test(21, 21, (42, false));
    test(i32::MIN, i32::MAX, (-1, false));
    test(-1, -i32::MAX, (i32::MIN, false));

    // test the overflowing addition
    test(i32::MAX, 1, (i32::MIN, true));
    test(i32::MIN, -1, (i32::MAX, true));

    f.exit();
    let f = p.finish_function(f);
    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn overflow_sub_works() {
    let ty =
        tuple_ty(&[(Size::ZERO, i32::get_type()), (size(4), bool::get_type())], size(8), align(4));
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let loc = f.declare_local_with_ty(ty);
    f.storage_live(loc);
    let mut test = |l: i32, r: i32, result: (i32, bool)| {
        f.assign(loc, overflow_sub(const_int(l), const_int(r)));
        f.assume(eq(load(field(loc, 0)), const_int(result.0)));
        f.assume(not(bool_xor(load(field(loc, 1)), const_bool(result.1))));
    };
    // test the non-overflowing subtraction
    test(-1, i32::MAX, (i32::MIN, false));
    test(-1, i32::MIN, (i32::MAX, false));
    test(53, 11, (42, false));

    // test the overflowing subtraction
    test(i32::MIN, 1, (i32::MAX, true));
    test(0, i32::MIN, (i32::MIN, true));

    f.exit();
    let f = p.finish_function(f);
    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn overflow_mul_works() {
    let ty =
        tuple_ty(&[(Size::ZERO, i32::get_type()), (size(4), bool::get_type())], size(8), align(4));
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let loc = f.declare_local_with_ty(ty);
    f.storage_live(loc);
    let mut test = |l: i32, r: i32, result: (i32, bool)| {
        f.assign(loc, overflow_mul(const_int(l), const_int(r)));
        f.assume(eq(load(field(loc, 0)), const_int(result.0)));
        f.assume(not(bool_xor(load(field(loc, 1)), const_bool(result.1))));
    };
    // test the non-overflowing multiplication
    test(-1, i32::MAX, (i32::MIN + 1, false));
    test(-(1 << 15), 1 << 16, (i32::MIN, false));
    test(6, 7, (42, false));

    // test the overflowing multiplication
    test(i32::MIN, -1, (i32::MIN, true));
    test(1 << 16, 1 << 16, (0, true));

    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn overflow_ill_formed_left() {
    let ty =
        tuple_ty(&[(Size::ZERO, i32::get_type()), (size(4), bool::get_type())], size(8), align(4));
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let loc = f.declare_local_with_ty(ty);
    f.storage_live(loc);
    f.assign(loc, overflow_add(const_bool(false), const_int(0_i32)));
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "BinOp::IntWithOverflow: invalid left type");
}

#[test]
fn overflow_ill_formed_right() {
    let ty =
        tuple_ty(&[(Size::ZERO, i32::get_type()), (size(4), bool::get_type())], size(8), align(4));
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let loc = f.declare_local_with_ty(ty);
    f.storage_live(loc);
    f.assign(loc, overflow_add(const_int(0_i32), const_bool(false)));
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "BinOp::IntWithOverflow: invalid right type");
}
