extern crate intrinsics;
use intrinsics::*;

include!("../helper/transmute.rs");

fn true_fn() -> bool { true }
fn false_fn() -> bool { false }

fn main() {
    if true_fn() {
        print(1);
    } else {
        print(0);
    }

    if false_fn() {
        print(11);
    } else {
        print(10);
    }
}
