extern crate intrinsics;
use intrinsics::*;

static mut X: (i64, i64) = (0, 0);

fn main() { unsafe {
    X.1 = 42;
    print(X.1);
}}
