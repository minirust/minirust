#![allow(integer_to_ptr_transmutes)]
use std::mem::transmute;

fn main() {
    unsafe {
        let _i  = *transmute::<usize, *const [i32; 0]>(1);
    }
}
