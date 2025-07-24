//@ compile-flags: --minimize-tree-borrows

use std::cell::Cell;

/// We get a pointer to an element (of type (Cell<i32>, i32)) of a slice,
/// then we add an offset and perform a write to the i32 field of the
/// previous element in the slice.

fn main() {
    let mut root = [(Cell::new(0), 1), (Cell::new(2), 3)];

    // We want to avoid creating any intermediate references to the entire array,
    // otherwise, when we try to write to the second field of an element, it will
    // trigger UB because it is a local write to Frozen.  This is why we use
    // `ptr::slice_from_raw_parts`.
    let ptr = &mut root as *mut _ as *mut (Cell<i32>, i32);
    let ptr = std::ptr::slice_from_raw_parts(ptr, 1);

    unsafe {
        // Looks like [(Cell::new(0), 1)].
        let slice = &*ptr;
        let slice_ptr = slice.as_ptr() as *const i32 as *mut i32;

        *slice_ptr.offset(0) = 100;
        // Ovewrite the 3 in (Cell::new(2), 3).
        *slice_ptr.offset(3) = 100;
    }

    assert!(root[0].0.get() == 100);
    assert!(root[1].1 == 100);
}
