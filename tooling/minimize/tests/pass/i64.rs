extern crate intrinsics;
use intrinsics::*;

fn main() {
    let mut x = 23i64;
    x += 3;
    x = double(x);
    print(x);
}

fn double(x: i64) -> i64 {
    x * 2
}
