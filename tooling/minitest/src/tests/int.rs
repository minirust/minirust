use crate::*;

/// Test that BinOpInt::BitAnd works for ints
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

// Test that BinOpInt::BitAnd fails with non-int/non-bool
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

// Test that BinOpInt::BitAnd fails with bool
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
