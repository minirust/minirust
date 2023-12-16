extern crate intrinsics;
use intrinsics::*;

fn true_fn() -> bool { true }
fn false_fn() -> bool { false }

fn one_fn() -> u8 { 1 }
fn two_fn() -> u8 { 2 }

fn main() {
    let tval = true_fn();
    match tval {
        true => print(1),
        false => print(0),
    }

    let fval = false_fn();
    match fval {
        true => print(11),
        false => print(10),
    }

    let oneval = one_fn();
    match oneval {
        1u8 => print(111),
        _ => print(110),
    }

    let twoval = two_fn();
    match twoval {
        1u8 => print(1111),
        _ => print(1110),
    }
}
