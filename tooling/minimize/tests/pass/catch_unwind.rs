#![feature(core_intrinsics)]
#![allow(internal_features)]

extern crate intrinsics;
use intrinsics::*;

#[allow(unconditional_panic)]
/// This function always panics. It has the signature of a try function.
fn try_panic(_data_ptr: *mut u8) {
    let _a = 5 / 0;
    print(-1);
}

/// This function increases the value at the given data pointer by 1. It has the signature of a try function.
fn try_increase_data(data_ptr: *mut u8) {
    unsafe {
        *data_ptr = (*data_ptr) + 1;
    }
}

/// This function uses `catch_unwind`. It can be used to test nested catch structures. It has the signature of a try function.
fn try_nested_catch_unwind(data_ptr: *mut u8) {
    unsafe {
        core::intrinsics::catch_unwind(try_panic, data_ptr, catch_print_data);
    }
}

/// This function prints the value at the given data pointer. It has the signature of a catch function.
fn catch_print_data(data_ptr: *mut u8, _payload: *mut u8) {
    unsafe {
        print(*data_ptr);
    }
}

/// This function increases the value at the given data pointer by 1. It has the signature of a catch function.
fn catch_increase_data(data_ptr: *mut u8, _payload: *mut u8) {
    unsafe {
        *data_ptr = (*data_ptr) + 1;
    }
}

/// This function uses `catch_unwind`. It can be used to test nested catch structures. It has the signature of a catch function.
fn catch_nested_catch_unwind(data_ptr: *mut u8, _payload: *mut u8) {
    unsafe {
        core::intrinsics::catch_unwind(try_panic, data_ptr, catch_print_data);
    }
}

/// This function causes undefined behavior when executed. It has the signature of a catch function.
fn catch_unreachable(_data_ptr: *mut u8, _payload: *mut u8) {
    unsafe {
        std::hint::unreachable_unchecked();
    }
}

/// This function prints the value at the given data pointer.
/// It is used to check when the expression used as the data pointer gets evaluated.
fn evaluate_data_ptr(data_ptr: *mut u8) -> *mut u8 {
    unsafe {
        print(*data_ptr);
    }
    data_ptr
}

fn main() {
    let mut data: u8 = 5;
    let data_ptr = &mut data as *mut u8;
    
    print(0);

    // As `try_panic` panics, `catch_print_data` will be executed.
    // This should print 5
    let mut ret = unsafe { core::intrinsics::catch_unwind(try_panic, data_ptr, catch_print_data) };
    assert!(ret == 1);
    assert!(data == 5);

    print(0);

    // `try_increase_data` does not panic, `catch_print_data` will not be executed.
    ret = unsafe { core::intrinsics::catch_unwind(try_increase_data, data_ptr, catch_unreachable) };
    assert!(ret == 0);
    assert!(data == 6); // data was increased by 1

    print(0);

    // The execution panics, however the panic gets caught inside `try_nested_catch_unwind`. `try_nested_catch_unwind` prints 6.
    ret = unsafe { core::intrinsics::catch_unwind(try_nested_catch_unwind, data_ptr, catch_unreachable) };
    assert!(ret == 0);
    assert!(data == 6);

    print(0);

    // `try_panic` panics. There is a panic in `catch_nested_catch_unwind`, however it will be caught inside `catch_nested_catch_unwind`
    // `catch_nested_catch_unwind` prints 6
    ret = unsafe { core::intrinsics::catch_unwind(try_panic, data_ptr, catch_nested_catch_unwind) };
    assert!(ret == 1);
    assert!(data == 6);

    print(0);


    // make sure the data_ptr expression is only evaluated once
    ret = unsafe { core::intrinsics::catch_unwind(try_panic, evaluate_data_ptr(data_ptr), catch_increase_data) };
    assert!(ret == 1);
    assert!(data == 7);

    print(0);
}
