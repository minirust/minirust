extern crate intrinsics;
use intrinsics::*;

static X: (i64, i64) = (2, 3);

fn main() {
    let x = X;
    print(x.0);
    print(x.1);
}
