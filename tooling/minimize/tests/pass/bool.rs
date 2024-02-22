extern crate intrinsics;
use intrinsics::*;

fn main() {
    if !true {
        print(-1);
    }
    if !false {}
    else {
        print(-1);
    }
}
