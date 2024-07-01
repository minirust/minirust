#![allow(internal_features)] 
#![feature(core_intrinsics)]
extern crate intrinsics;
use intrinsics::*;

fn main() {
    // use `unchecked_div` to avoid `assert` != 0 before division
    print(unsafe { std::intrinsics::unchecked_div(1, black_box(0)) });
}

fn black_box<T>(t: T) -> T { t }
