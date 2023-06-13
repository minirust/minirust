extern crate intrinsics;
use intrinsics::*;

fn main() {
    print(1 / black_box(0));
}

fn black_box<T>(t: T) -> T { t }
