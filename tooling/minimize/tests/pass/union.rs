extern crate intrinsics;
use intrinsics::*;

union A {
    f1: u32,
    f2: (),
}

fn main() {
    let mut x = A { f2: ()};
    x.f1 = 20;
    unsafe {
        print(x.f1);
    }
}
