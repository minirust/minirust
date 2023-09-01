extern crate intrinsics;
use intrinsics::*;

#[repr(packed)]
struct P {
    field: u32,
}

fn main() {
    let mut p = P { field: 0 };
    p.field = 42;
    print(p.field);
}
