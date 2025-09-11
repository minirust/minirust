//@ compile-flags: --minimize-tree-borrows

// Check that deallocating an allocation is allowed even in the presence of a strong protector only covering interior mutable data.

extern crate intrinsics;
use intrinsics::*;
use std::cell::Cell;

// `x` is strongly protected but covers only `Cell` bytes.
fn foo(_x: &Cell<u8>, ptr: *mut u8) {
    unsafe { deallocate(ptr, 1, 1) };
}

fn main() {
    let raw = unsafe { allocate(1, 1) };
    let x = unsafe { &mut *(raw as *mut Cell<u8>) };

    foo(x, raw);
}
