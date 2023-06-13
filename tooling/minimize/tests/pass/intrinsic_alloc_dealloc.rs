extern crate intrinsics;
use intrinsics::*;

fn main() {
    unsafe {
        let ptr = allocate(1, 1);
        *ptr = 24;
        print(*ptr);
        deallocate(ptr, 1, 1);
    }
}

