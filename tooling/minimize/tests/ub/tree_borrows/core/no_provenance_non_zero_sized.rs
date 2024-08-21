//@ compile-flags: --minimize-tree-borrows

// Check that we forbid retagging a non-zero-sized pointer without provenance.

#![feature(strict_provenance)]

fn foo(_x: &mut u8) {
    // UB! Retagging a non-zero-sized pointer without provenance.
}

fn main() {
    let xraw = &mut 42u8 as *mut u8;
    let ptr = std::ptr::without_provenance_mut::<u8>(xraw.addr());
    foo(unsafe { &mut *ptr });
}
