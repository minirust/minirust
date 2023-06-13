extern crate intrinsics;
use intrinsics::*;

fn main() {
    let mut x = 2;
    let y = &mut x as *mut i32;
    unsafe { *y = 3; }
    print(x);
}
