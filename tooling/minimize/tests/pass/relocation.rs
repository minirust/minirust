extern crate intrinsics;
use intrinsics::*;

static A: i32 = 2;
static X: (i64, &i32, i64) = (0, &A, 3);

fn main() {
    let x = X;
    print(*x.1);
    print(x.2);
}
