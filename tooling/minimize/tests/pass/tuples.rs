extern crate intrinsics;
use intrinsics::*;

fn main() {
    let x = 30;
    print(foo(x-12).1);

    // Also test tuple comparison.
    assert!((1, 0) != (0, 1));
    assert!((1, 0) == (1, 0));
}

fn foo(x: i32) -> (i32, i32) {
    print(x);
    (x+1, x+2)
}
