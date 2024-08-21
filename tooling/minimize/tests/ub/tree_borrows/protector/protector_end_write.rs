//@ compile-flags: --minimize-tree-borrows

// The test was taken from Miri Tree Borrows 
// https://github.com/rust-lang/miri/blob/master/tests/fail/tree_borrows/protector-write-lazy.rs

// This test tests that TB's protector end semantics correctly ensure
// that protected activated writes can be reordered.
fn the_other_function(ref_to_fst_elem: &mut u8, ptr_to_vec: *mut u8) -> *mut u8 {
    // Activate the reference. Afterwards, we should be able to reorder arbitrary writes.
    *ref_to_fst_elem = 0;
    // Here is such an arbitrary write.
    // It could be moved down after the retag, in which case the `funky_ref` would be invalidated.
    // We need to ensure that the `funky_ptr` is unusable even if the write to `ref_to_fst_elem`
    // happens before the retag.
    *ref_to_fst_elem = 42;
    // this creates a reference that is Reserved Lazy on the first element (offset 0).
    // It does so by doing a proper retag on the second element (offset 1), which is fine
    // since nothing else happens at that offset, but the lazy init mechanism means it's
    // also reserved at offset 0, but not initialized.
    let funky_ptr_lazy_on_fst_elem = unsafe { ((&mut *(ptr_to_vec.add(1))) as *mut u8).sub(1) };

    // If we write to `ref_to_fst_elem` here, then any further access to `funky_ptr_lazy_on_fst_elem` would
    // definitely be UB. Since the compiler ought to be able to reorder the write of `42` above down to
    // here, that means we want this program to also be UB.
    return funky_ptr_lazy_on_fst_elem;
}

fn main() {
    let mut v = [0u8, 1];
    // get a pointer to the root of the allocation
    // note that it's not important it's the actual root, what matters is that it's a parent
    // of both references that will be created
    let ptr_to_vec = &mut v[0] as *mut u8;
    let ref_to_fst_elem = unsafe { &mut *ptr_to_vec };
    let funky_ptr_lazy_on_fst_elem = the_other_function(ref_to_fst_elem, ptr_to_vec);
    // now we try to use the funky lazy pointer.
    // It should be UB, since the write-on-protector-end should disable it.
    unsafe { assert!(*funky_ptr_lazy_on_fst_elem == 42); } 
}
