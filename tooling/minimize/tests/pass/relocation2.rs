extern crate intrinsics;
use intrinsics::*;

static A: [i32; 2] = [100, 2];
static X: (i8, &i32, i64) = (0, &A[1], 3);

fn main() {
    let x = X;
    print(*x.1);
    print(x.2);
}
