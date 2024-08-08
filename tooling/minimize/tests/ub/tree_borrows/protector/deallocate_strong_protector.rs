//@ compile-flags: --minimize-tree-borrows

// Check that deallocating an allocation contains strongly protected node is UB

extern crate intrinsics;
use intrinsics::*;

fn foo(x: &mut u8) {
    unsafe { deallocate(x as *mut u8, 1, 1); } 
}

fn main() {
    let xraw = unsafe { allocate(1, 1) } ;
    let x = unsafe { &mut *xraw };

    foo(x);
}
