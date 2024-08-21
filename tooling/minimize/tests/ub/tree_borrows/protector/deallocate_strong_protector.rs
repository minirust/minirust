//@ compile-flags: --minimize-tree-borrows

// Check that deallocating an allocation containing a strongly protected node is UB.
// FIXME: Add test for deallocating weak protectors after `Box` is supported.

extern crate intrinsics;
use intrinsics::*;

fn foo(x: &mut u8) {
    unsafe { deallocate(x as *mut u8, 1, 1) };
}

fn main() {
    let xraw = unsafe { allocate(1, 1) } ;
    let x = unsafe { &mut *xraw };

    foo(x);
}
