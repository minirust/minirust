extern crate intrinsics;
use intrinsics::*;

//@ compile-flags: --minimize-tree-borrows
fn main() {
    let parent = &mut 0u8; // (parent, Reserved)   
    let x = &mut *parent; // (parent, Reserved) -> (x, Reserved)
    let y = &mut *x; // (parent, Reserved) -> (x, Reserved) -> (y, Reserved)
       
    *y = 42; // (parent, Reserved) -> (x, Reserved) -> (y, Active)
    *x = 31; // (parent, Reserved) -> (x, Active) -> (y, Disable)
    
    print(*x);
}
