extern crate intrinsics;
use intrinsics::*;

fn main() {
    let x = dangling();
    let y = unsafe { *x };
    print(y);
}

fn dangling() -> *const i32 {
    let x = 2;
    &x as *const i32
}
