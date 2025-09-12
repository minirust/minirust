//@ compile-flags: --minimize-tree-borrows

// test that protector end writes don't trigger for non-accessed or `Cell` bytes.
use std::cell::Cell;

fn other_function_zero_sized(ref_to_fst_elem: &mut (), ptr_to_vec: *mut u8) -> *mut u8 {
    // ref_to_fst_elem is zero-sized so really nothing should happen at the protector end write.
    *ref_to_fst_elem = ();
    // this creates a reference that is Reserved Lazy on the first element (offset 0).
    // It does so by doing a proper retag on the second element (offset 1), which is fine
    // since nothing else happens at that offset, but the lazy init mechanism means it's
    // also reserved at offset 0, but not initialized.
    let funky_ptr_lazy_on_fst_elem = unsafe { ((&mut *(ptr_to_vec.add(1))) as *mut u8).sub(1) };
    return funky_ptr_lazy_on_fst_elem;
}

fn check_zero_sized() {
    let mut v = [42u8, 1];
    // get a pointer to the root of the allocation
    // note that it's not important it's the actual root, what matters is that it's a parent
    // of both references that will be created
    let ptr_to_vec = &mut v[0] as *mut u8;
    let ref_to_fst_elem: &mut () = unsafe { &mut *(ptr_to_vec as *mut ()) };
    let funky_ptr_lazy_on_fst_elem = other_function_zero_sized(ref_to_fst_elem, ptr_to_vec);
    // now we try to use the funky lazy pointer.
    // It should be allowed, since no protector end write invalidated it.
    unsafe {
        assert!(*funky_ptr_lazy_on_fst_elem == 42);
    }
}

fn other_function_cell(ref_to_fst_elem: &Cell<u8>, ptr_to_vec: *mut u8) -> *mut u8 {
    // ref_to_fst_elem is `Cell` so the protector does not actually apply to it.
    // Even if we write to it, no protector end actions should be triggered.
    ref_to_fst_elem.set(42u8);
    // this creates a reference that is Reserved Lazy on the first element (offset 0).
    // It does so by doing a proper retag on the second element (offset 1), which is fine
    // since nothing else happens at that offset, but the lazy init mechanism means it's
    // also reserved at offset 0, but not initialized.
    let funky_ptr_lazy_on_fst_elem = unsafe { ((&mut *(ptr_to_vec.add(1))) as *mut u8).sub(1) };
    return funky_ptr_lazy_on_fst_elem;
}

fn check_cell() {
    let mut v = [0u8, 1];
    // get a pointer to the root of the allocation
    // note that it's not important it's the actual root, what matters is that it's a parent
    // of both references that will be created
    let ptr_to_vec = &mut v[0] as *mut u8;
    let ref_to_fst_elem: &Cell<u8> = Cell::from_mut(unsafe { &mut *ptr_to_vec });
    let funky_ptr_lazy_on_fst_elem = other_function_cell(ref_to_fst_elem, ptr_to_vec);
    // now we try to use the funky lazy pointer.
    // It should be allowed, since no protector end write invalidated it.
    unsafe {
        assert!(*funky_ptr_lazy_on_fst_elem == 42);
    }
}

fn main() {
    check_zero_sized();
    check_cell();
}
