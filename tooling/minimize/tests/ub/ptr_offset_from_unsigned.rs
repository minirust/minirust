#![allow(internal_features)]
#![feature(ptr_sub_ptr, core_intrinsics)]

use std::intrinsics;

fn main() {
    let data = [1u8, 2, 3, 4, 5];
    unsafe {
        // Negative offset in `ptr_offset_from_unsigned` is UB.
        intrinsics::ptr_offset_from_unsigned(&data[0], &data[4]);
    }
}
