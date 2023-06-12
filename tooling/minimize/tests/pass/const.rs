extern crate intrinsics;
use intrinsics::*;

const X: (i64, i64) = (2, 3);

fn main() {
    let x = X;
    print(x.0);
    print(x.1);
}
