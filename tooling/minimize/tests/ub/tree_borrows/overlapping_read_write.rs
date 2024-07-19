extern crate intrinsics;
use intrinsics::*;

//@ compile-flags: --minimize-tree-borrows
fn main() {
    let parent = &mut 31; // (parent, Reserved)   
    let x = parent as *mut i32; // (parent, Reserved) 
    let y = unsafe { &mut *x }; // (parent, Reserved) -> (y, Reserved)
    *y = 42;
    unsafe { print(*x); } // (parent, Reserved) -> (y, Frozen)
    *y = 31; // UB! Child Write to Frozen 
}
