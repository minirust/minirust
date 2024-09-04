#![feature(ptr_sub_ptr)]

use std::ptr;

fn main() {
    let data = [1u8, 2, 3, 4, 5];
    unsafe {
        // Negative offset in `sub_ptr` is UB.
        ptr::from_ref(&data[0]).sub_ptr(&data[4]);
    }
}
