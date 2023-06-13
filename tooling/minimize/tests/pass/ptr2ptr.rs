extern crate intrinsics;
use intrinsics::*;

fn main() {
    let x = -1;
    let y = &x as *const i32 as *const u32;
    print(unsafe { *y });
    print(u32::MAX);
}
