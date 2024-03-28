extern crate intrinsics;
use intrinsics::*;

union A {
    f1: u32,
    f2: (),
}

#[allow(unused)]
union B {
    f1: (u8, u16),
    f2: u8,
}

fn main() {
    let mut x = A { f2: ()};
    x.f1 = 20;
    unsafe {
        print(x.f1);
    }

    let _y = B { f2: 0 };
}
