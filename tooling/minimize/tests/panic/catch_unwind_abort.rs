//@ compile-flags: -C panic=abort
#![feature(core_intrinsics)]
#![allow(internal_features)]

extern crate intrinsics;
use intrinsics::*;

#[allow(unconditional_panic)]
fn try_fn(_data_ptr: *mut u8) {
    print(1);
    let _x = 5 / 0;
    print(2);
}

fn catch_fn(_data_ptr: *mut u8, _payload: *mut u8) {
    print(3);
}

fn main() {
    let mut data: u8 = 0;
    let data_ptr = &mut data as *mut u8;
    print(0);
    unsafe {
        // This does not catch the panic, as panics are set to abort.
        core::intrinsics::catch_unwind(try_fn, data_ptr, catch_fn);
    }
    print(4);
}
