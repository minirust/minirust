use crate::*;

#[test]
fn assume_true() {
    let locals = [];
    let b0 = block!(assume(const_bool(true), 1));
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_stop::<BasicMem>(p);
}

#[test]
fn assume_false() {
    let locals = [];
    let b0 = block!(assume(const_bool(false), 1));
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub::<BasicMem>(p, "`Assume` intrinsic called on condition that is violated");
}

#[test]
fn assume_wrong_argnum() {
    let locals = [];
    let b0 = block!(Terminator::Intrinsic {
        intrinsic: IntrinsicOp::Assume,
        arguments: list![], // no arguments
        ret: zst_place(),
        next_block: Some(BbName(Name::from_internal(1))),
    });
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub::<BasicMem>(p, "invalid number of arguments for `Assume` intrinsic");
}

#[test]
fn assume_wrong_argty() {
    let locals = [];
    let b0 = block!(Terminator::Intrinsic {
        intrinsic: IntrinsicOp::Assume,
        arguments: list![const_int::<i32>(0)], // should be bool, not int
        ret: zst_place(),
        next_block: Some(BbName(Name::from_internal(1))),
    });
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub::<BasicMem>(p, "invalid argument for `Assume` intrinsic: not a Boolean");
}
