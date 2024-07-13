#![allow(internal_features)] 
#![feature(core_intrinsics)]
extern crate intrinsics;
use intrinsics::*;

fn main() {
    // Use `unchecked_div` to make div-by-zero UB (rather than panic).
    print(unsafe { std::intrinsics::unchecked_div(1, black_box(0)) });
}

fn black_box<T>(t: T) -> T { t }
