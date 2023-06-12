extern crate intrinsics;
use intrinsics::*;

struct A {
    x: u32,
}

fn main() {
    let mut a = A { x: 20 };
    a.x += 1;
    print(a.x);
}
