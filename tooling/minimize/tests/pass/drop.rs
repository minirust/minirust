extern crate intrinsics;
use intrinsics::*;

struct Bomb;

impl Drop for Bomb {
    fn drop(&mut self) {
        print(42);
    }
}

fn main() {
    let _b = Bomb;
}
