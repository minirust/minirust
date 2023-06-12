extern crate intrinsics;
use intrinsics::*;

// This creates an uninitialized gap between the two.
const X: (i32, i64) = (2, 3);

fn main() {
    let x = X;
    print(x.0);
    print(x.1);
}
