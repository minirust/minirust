#![feature(core_intrinsics)]
#![allow(internal_features)]

#[allow(unconditional_panic)]
fn try_fn(_data_ptr: *mut u8) {
    assert!(false);
}

#[allow(unconditional_panic)]
fn catch_fn(_data_ptr: *mut u8, _payload: *mut u8) {
    assert!(false);
}

fn main() {
    let mut data: u8 = 0;
    let data_ptr = &mut data as *mut u8;
    unsafe {
        // Both `try_fn` and `catch_fn` panic. This results in UB.
        core::intrinsics::catch_unwind(try_fn, data_ptr, catch_fn);
    }
}
