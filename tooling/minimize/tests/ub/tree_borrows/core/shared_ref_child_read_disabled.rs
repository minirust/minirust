//@ compile-flags: --minimize-tree-borrows

// Check that a foreign write makes an frozen shared reference disabled.
// After that, reading from this shared reference is UB.

fn main() {
    let x = &mut 31; // (x, Reserved)   
    let xraw = x as *mut i32; // (x, Reserved) 
    let y = unsafe { &*xraw }; // (x, Reserved) -> (y, Frozen)
    
    unsafe { *xraw = 42; } // (x, Active) -> (y, Disable)

    assert!(*y == 42); // UB! Child read from Disabled
}
