extern crate intrinsics;
use intrinsics::*;

fn main() {
    let x = 2;
    let y = &x;
    print(*y);
}
