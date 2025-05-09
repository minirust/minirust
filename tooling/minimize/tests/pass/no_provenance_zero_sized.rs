//@ compile-flags: --minimize-tree-borrows

// Check that we allow retagging a zero-sized pointer without provenance.

fn foo(_x: &mut ()) {
    // It is okay to retag the pointer, because the pointee is zero-sized.
}

fn main() {
    let xraw = &mut 42u8 as *mut u8;
    let ptr = std::ptr::without_provenance_mut::<()>(xraw.addr());
    foo(unsafe { &mut *ptr });
}
