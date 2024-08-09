//@ compile-flags: --minimize-tree-borrows

// This test was taken from Miri Tree Borrows
// https://github.com/rust-lang/miri/blob/master/tests/pass/tree_borrows/transmute-unsafecell.rs

//! Testing `mem::transmute` between types with and without interior mutability.
//! All transmutations should work, as long as we don't do any actual accesses
//! that violate immutability.

include!("../../helper/transmute.rs");
use core::cell::UnsafeCell;


fn main() {
    unsafe {
        ref_to_cell();
        cell_to_ref();
    }
}

// Pretend that the reference has interior mutability.
// Don't actually mutate it though, it will fail because it has a Frozen parent.
unsafe fn ref_to_cell() {
    let x = &42i32;
    let cell_x: &UnsafeCell<i32> = transmute(x);
    let val = *cell_x.get();
    assert!(val == 42);
}

// Forget about the interior mutability of a cell.
unsafe fn cell_to_ref() {
    let x = &UnsafeCell::new(42);
    let ref_x: &i32 = transmute(x);
    let val = *ref_x;
    assert!(val == 42);
}
