//@ compile-flags: --minimize-tree-borrows

// Check that deallocating an allocation is allowed even in the presence of a zero-sized strong protector.

extern crate intrinsics;
use intrinsics::*;

// `x` is strongly protected but covers zero bytes.
fn foo(_x: &mut (), ptr: *mut u8) {
    unsafe { deallocate(ptr, 1, 1) };
}

fn main() {
    let raw = unsafe { allocate(1, 1) };
    let x = unsafe { &mut *(raw as *mut ()) };

    foo(x, raw);
}
