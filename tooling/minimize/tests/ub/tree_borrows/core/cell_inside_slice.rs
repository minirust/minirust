//@ compile-flags: --minimize-tree-borrows

use std::cell::Cell;

fn main() {
    let root = &[(Cell::new(0), 1), (Cell::new(2), 3)][..];
    let x = &root[0];
    let x: *mut (Cell<i32>, i32) = x as *const _ as *mut _;
    unsafe {
        // The first element allows interior mutablity, but the
        // second element of the tuple is frozen.
        (*x).1 = 42; // UB! Child write to frozen.
    }
}
