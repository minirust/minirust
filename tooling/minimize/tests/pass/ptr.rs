extern crate intrinsics;
use intrinsics::*;

fn main() {
    let x = 2;
    let y = &x as *const i32;
    let z = unsafe { *y };
    print(z);
}
