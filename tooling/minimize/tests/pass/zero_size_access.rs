//@revisions: basic tree
//@[tree]compile-flags: --minimize-tree-borrows

// Check that we allow out-of-range zero-sized accesses and reborrows.

fn main() {
    let xraw = &mut 42u8 as *mut u8;
    let xzst = xraw.wrapping_add(2) as *mut ();
    unsafe { assert!(*xzst == ()) }; // read
    unsafe { *xzst = () }; // write
    let _reborrow = unsafe { &mut *xzst }; // reborrow
}
