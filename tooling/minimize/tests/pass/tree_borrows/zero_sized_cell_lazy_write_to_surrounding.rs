//@ compile-flags: --minimize-tree-borrows

use std::cell::Cell;

/// Variant of `lazy_write_to_surrounding.rs` but with zero-sized UnsafeCells.
/// Zero-sized UnsafeCells should also allow mutation to the "outside" bytes.

fn main() {
    let mut arr = [(Cell::new(()), 0), (Cell::new(()), 1), (Cell::new(()), 2)];

    // We want to avoid creating any intermediate references to the entire array,
    // otherwise, when we try to write to an element, it will trigger UB because
    // it is a local write to Frozen.  This is why we use `ptr::slice_from_raw_parts`.
    let ptr = &mut arr as *mut _ as *mut (Cell<()>, i32);
    let ptr = std::ptr::slice_from_raw_parts(ptr, 1);

    unsafe {
        let slice = &*ptr;
        let slice_ptr = slice.as_ptr() as *const i32 as *mut i32;
        // Overwrite to second element of array.
        *slice_ptr.offset(1) = 100;
    }

    assert!(arr[1].1 == 100);
}
