//@ compile-flags: --minimize-tree-borrows

// This test was taken from Miri Tree Borrows
// https://github.com/rust-lang/miri/blob/master/tests/pass/tree_borrows/spurious_read.rs

extern crate intrinsics;
use intrinsics::*;

#[derive(Clone, Copy)]
struct ThreadData {
    counter_ptr: *mut u32,
    data_ptr: *mut u8,
}

// This thread's job is to
// - retag `y` protected
// - (wait a bit that the other thread performs its spurious read)
// - remove `y`'s protector
// - attempt a write through `y`.
extern "C" fn thread(thread_data_ptr: *const ()) {
    let ThreadData { counter_ptr, data_ptr }  = unsafe { *(thread_data_ptr as *const ThreadData) };

    fn foo(y: &mut u8, counter_ptr: *mut u32) -> *mut u8 {
        assert!(unsafe { atomic_fetch_add(counter_ptr, 1) } == 1); // 1 -> 2
        while unsafe { atomic_load(counter_ptr) == 2 } {}
        let y = y as *mut u8;
        y
    }

    while unsafe { atomic_load(counter_ptr) == 0 } {}
    let y = foo(unsafe { &mut *data_ptr }, counter_ptr);
    unsafe { *y = 2 };
}

// Interleaving:
//   retag x (protect)
//   retag y (protect)
//   spurious read x (target only, which we are executing)
//   ret x
//   ret y
//   write y
//
// This is an interleaving that will never have UB in the source
// (`x` is never accessed for the entire time that `y` is protected).
// For the spurious read to be allowed, we need to check that there is
// no UB in the target (i.e., *with* the spurious read).
//
// The interleaving differs from the one in `ub/tree_borrows/protector/protector_end_read.rs` only
// in that it has the `write y` while `y` is no longer protected.
// When the write occurs after protection ends, both source and target are fine
// (checked by this test); when the write occurs during protection, both source
// and target are UB (checked by the `fail` test).
fn main() {
    let fn_ptr =  thread as extern "C" fn(*const ());

    let mut data = 0u8;
    let data_ptr = std::ptr::addr_of_mut!(data);

    let mut counter = 0u32;
    let counter_ptr = std::ptr::addr_of_mut!(counter);

    let thread_data = ThreadData { counter_ptr, data_ptr };
    let thread_data_ptr = &thread_data as *const ThreadData as *const ();

    let thread_id = spawn(fn_ptr, thread_data_ptr);

    // The main thread only needs to
    // - retag `x` protected
    // - do a read through `x`
    // - remove `x`'s protector
    // Most of the complexity here is synchronization.
    fn foo(x: &mut u8, counter_ptr: *mut u32) -> *mut u8 {
        assert!(unsafe { atomic_fetch_add(counter_ptr, 1) } == 0); // 0 -> 1
        while unsafe { atomic_load(counter_ptr) == 1 } {}
        // This is ensuring taht we have UB *without* the spurious read,
        // so we don't read here.
        let x = x as *mut u8;
        unsafe { assert!(*x == 0) };
        x
    }
    foo(unsafe { &mut *data_ptr }, counter_ptr);
    assert!(unsafe { atomic_fetch_add(counter_ptr, 1) } == 2); // 2 -> 3
    
    join(thread_id);
}
