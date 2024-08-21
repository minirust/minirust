//@ compile-flags: --minimize-tree-borrows
//@revisions: without with

// This test was taken from Miri Tree Borrows
// https://github.com/rust-lang/miri/blob/master/tests/fail/tree_borrows/reservedim_spurious_write.rs

// Illustrating a problematic interaction between Reserved, interior mutability, and protectors.
// This explains why protected mutable references to `!Freeze` start in `Reserved`, not `ReservedIM`.

// One revision without spurious read (default source code) and one with spurious read.
// Both are expected to be UB. Both revisions are expected to have the *same* error
// because we are aligning the behavior of `without` to that of `with` so that the
// spurious write is effectively a noop in the long term.

extern crate intrinsics;
use intrinsics::*;
use std::cell::Cell;

#[derive(Clone, Copy)]
struct ThreadData {
    counter_ptr: *mut u32,
    data_ptr: *mut u8,
}

// Create a lazy Reserved with interior mutability.
// Wait for the other thread's spurious write then observe the side effects
// of that write.
extern "C" fn thread(thread_data_ptr: *const ()) {
    let ThreadData { counter_ptr, data_ptr }  = unsafe { *(thread_data_ptr as *const ThreadData) };

    fn inner(y: &mut Cell<()>, counter_ptr: *mut u32) -> *mut u8 {
        assert!(unsafe { atomic_fetch_add(counter_ptr, 1) } == 1); // 1 -> 2
        while unsafe { atomic_load(counter_ptr) == 2 } {}
        // `y` is not retagged for any bytes, so the pointer we return
        // has its permission lazily initialized.
        y as *mut Cell<()> as *mut u8
    }

    while unsafe { atomic_load(counter_ptr) == 0 } {}
    let y_zst = unsafe { &mut *(data_ptr as *mut Cell<()>) };
    let y = inner(y_zst, counter_ptr);
    assert!(unsafe { atomic_fetch_add(counter_ptr, 1) } == 3); // 3 -> 4
    while unsafe { atomic_load(counter_ptr) == 4 } {}
    unsafe { *y = 13 }
}

// Here is the problematic interleaving:
// - thread 1: retag and activate `x` (protected)
// - thread 2: retag but do not initialize (lazy) `y` as Reserved with interior mutability
// - thread 1: spurious write through `x` would go here
// - thread 2: function exit (noop due to lazyness)
// - thread 1: function exit (no permanent effect on `y` because it is now Reserved IM unprotected)
// - thread 2: write through `y`
// In the source code nothing happens to `y`
fn main() {
    let fn_ptr =  thread as extern "C" fn(*const ());

    let mut data = 0u8;
    let data_ptr = std::ptr::addr_of_mut!(data);

    let mut counter = 0u32;
    let counter_ptr = std::ptr::addr_of_mut!(counter);

    let thread_data = ThreadData { counter_ptr, data_ptr }; 
    let thread_data_ptr = &thread_data as *const ThreadData as *const ();

    let thread_id = spawn(fn_ptr, thread_data_ptr);

    // Retag and activate `x`, wait until the other thread creates a lazy permission.
    // Then do a spurious write. Finally exit the function after the other thread.
    fn inner(x: &mut u8, counter_ptr: *mut u32) {
        *x = 42; // activate immediately
        assert!(unsafe { atomic_fetch_add(counter_ptr, 1) } == 0);
        while unsafe { atomic_load(counter_ptr) == 1 } {}
        // A spurious write should be valid here because `x` is
        // `Active` and protected.
        if cfg!(with) { *x = 64 };
        assert!(unsafe { atomic_fetch_add(counter_ptr, 1) } == 2); // 2 -> 3
        while unsafe { atomic_load(counter_ptr) == 3 } {}
    }
    inner(unsafe { &mut *data_ptr }, counter_ptr);
    assert!(unsafe { atomic_fetch_add(counter_ptr, 1) } == 4); // 4 -> 5

    join(thread_id);
}
