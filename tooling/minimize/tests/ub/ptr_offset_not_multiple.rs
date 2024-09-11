#![feature(ptr_sub_ptr)]

use std::ptr;

fn main() {
    let data = [1u16, 2, 3, 4, 5];
    unsafe {
        // Negative offset in `sub_ptr` is UB.
        let ptr = ptr::from_ref(&data[0]).cast::<u8>().add(1).cast::<u16>();
        ptr::from_ref(&data[4]).sub_ptr(ptr);
    }
}
