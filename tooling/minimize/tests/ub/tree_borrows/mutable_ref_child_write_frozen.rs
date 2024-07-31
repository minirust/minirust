//@ compile-flags: --minimize-tree-borrows

// Check that a foreign read makes an active mutable reference frozen.
// After that, writing to this mutable reference is UB.

fn main() {
    let x = &mut 31; // (x, Reserved)   
    let xraw = x as *mut i32; // (x, Reserved) 
    let y = unsafe { &mut *xraw }; // (x, Reserved) -> (y, Reserved)
    *y = 42; // (x, Active) -> (y, Active)
    assert!(unsafe { *xraw } == 42); // (x, Active) -> (y, Frozen)

    assert!(*y == 42); // Child read from Frozen is ok.
    *y = 31; // UB! Child Write to Frozen.
}
