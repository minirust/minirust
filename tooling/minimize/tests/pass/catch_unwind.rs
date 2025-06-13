#![feature(core_intrinsics)]
#![allow(internal_features)]
#![allow(dead_code)]

extern crate intrinsics;
use intrinsics::*;

/// This function causes undefined behavior when executed. It has the signature of a catch function.
fn catch_unreachable(_data_ptr: *mut u8, _payload: *mut u8) {
    unsafe {
        std::hint::unreachable_unchecked();
    }
}

/// Test `catch_unwind` when `try_fn` does not panic.
fn test_no_panic() {
    let mut data: u8 = 0;
    let data_ptr = &mut data as *mut u8;

    fn try_fn(data_ptr: *mut u8) {
        unsafe {
            *data_ptr = (*data_ptr) + 10;
        }
    }

    let ret = unsafe { core::intrinsics::catch_unwind(try_fn, data_ptr, catch_unreachable) };

    assert!(data == 10);
    assert!(ret == 0);
}

/// Test `catch_unwind` when `try_fn` panics.
fn test_panic_in_try_fn() {
    let mut data: u8 = 0;
    let data_ptr = &mut data as *mut u8;

    #[allow(unconditional_panic)]
    fn try_fn(data_ptr: *mut u8) {
        unsafe {
            *data_ptr = (*data_ptr) + 10;
        }

        // panics
        let _a = 5 / 0;
    }

    fn catch_fn(data_ptr: *mut u8, _payload: *mut u8) {
        unsafe {
            *data_ptr = (*data_ptr) + 1;
        }
    }

    let ret = unsafe { core::intrinsics::catch_unwind(try_fn, data_ptr, catch_fn) };

    assert!(data == 11);
    assert!(ret == 1);
}

/// This test triggers a panic inside `try_fn`. However, the panic is caught within `try_fn`,
/// so `catch_fn` should not be executed.
fn test_nested_catch_in_try_fn() {
    let mut data: u8 = 0;
    let data_ptr = &mut data as *mut u8;

    #[allow(unconditional_panic)]
    fn inner_try(data_ptr: *mut u8) {
        unsafe {
            *data_ptr = (*data_ptr) + 1;
        }

        //panics
        assert!(false);
    }

    fn inner_catch(data_ptr: *mut u8, _payload: *mut u8) {
        unsafe {
            *data_ptr = (*data_ptr) + 10;
        }
    }

    fn outer_try(data_ptr: *mut u8) {
        unsafe {
            *data_ptr = (*data_ptr) + 50;
            core::intrinsics::catch_unwind(inner_try, data_ptr, inner_catch);
            *data_ptr = (*data_ptr) + 100;
        }
    }

    let ret = unsafe { core::intrinsics::catch_unwind(outer_try, data_ptr, catch_unreachable) };

    assert!(data == 161);
    assert!(ret == 0);
}

/// This test triggers a panic inside `catch_fn`. However, the panic is caught within `catch_fn`,
/// so there should be no undefined behavior
fn test_nested_catch_in_catch_fn() {
    let mut data: u8 = 0;
    let data_ptr = &mut data as *mut u8;

    #[allow(unconditional_panic)]
    fn inner_try(data_ptr: *mut u8) {
        unsafe {
            *data_ptr = (*data_ptr) + 1;
        }

        //panics
        assert!(false);
    }

    fn inner_catch(data_ptr: *mut u8, _payload: *mut u8) {
        unsafe {
            *data_ptr = (*data_ptr) + 10;
        }
    }

    #[allow(unconditional_panic)]
    fn outer_try(data_ptr: *mut u8) {
        unsafe {
            *data_ptr = (*data_ptr) + 50;
        }

        //panics
        assert!(false);
    }

    fn outer_catch(data_ptr: *mut u8, _payload: *mut u8) {
        unsafe {
            *data_ptr = (*data_ptr) + 70;
            core::intrinsics::catch_unwind(inner_try, data_ptr, inner_catch);
            *data_ptr = (*data_ptr) + 100;
        }
    }

    let ret = unsafe { core::intrinsics::catch_unwind(outer_try, data_ptr, outer_catch) };

    assert!(data == 231);
    assert!(ret == 1);
}

/// Test the order in which the arguments and the return place of `catch_unwind` are evaluated.
fn test_evaluation_order() {
    // Functions to simulate the evaluation of the arguments and the return value of `catch_unwind`.
    // Print statements are used to log the order of evaluation.

    fn eval_ret(ret_ptr: *mut i32) -> *mut i32 {
        print(4);
        ret_ptr
    }

    fn eval_try(try_fn: fn(*mut u8)) -> fn(*mut u8) {
        print(1);
        try_fn
    }

    fn eval_data(data_ptr: *mut u8) -> *mut u8 {
        print(2);
        data_ptr
    }

    fn eval_catch(catch_fn: fn(*mut u8, *mut u8)) -> fn(*mut u8, *mut u8) {
        print(3);
        catch_fn
    }

    fn try_fn(data_ptr: *mut u8) {
        unsafe {
            *data_ptr = (*data_ptr) + 1;
        }
        assert!(false);
    }

    fn catch_fn(data_ptr: *mut u8, _payload: *mut u8) {
        unsafe {
            *data_ptr = (*data_ptr) + 10;
        }
    }

    let mut data: u8 = 0;
    let data_ptr = &mut data as *mut u8;
    let mut ret: i32 = 0;
    let ret_ptr = &mut ret as *mut i32;

    print(0);
    unsafe {
        *(eval_ret(ret_ptr)) = core::intrinsics::catch_unwind(
            eval_try(try_fn),
            eval_data(data_ptr),
            eval_catch(catch_fn),
        );
    }
    print(5);

    assert!(data == 11);
    assert!(ret == 1);
}

fn main() {
    test_no_panic();
    test_panic_in_try_fn();
    test_nested_catch_in_try_fn();
    test_nested_catch_in_catch_fn();
    test_evaluation_order();
}
