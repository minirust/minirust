//@ compile-flags: --minimize-tree-borrows

// Check that a foreign write makes an active mutable reference disabled.
// After that, writing to this mutable reference is UB.

fn main() {
    let x = &mut 31; // (x, Reserved)   
    let xraw = x as *mut i32; // (x, Reserved) 
    let y = unsafe { &mut *xraw }; // (x, Reserved) -> (y, Reserved)
    *y = 42; // (x, Active) -> (y, Active)

    unsafe { 
        *xraw = 57; // (x, Active) -> (y, Disabled)
        assert!(*xraw == 57); // (x, Active) -> (y, Disabled)
    } 

    *y = 31; // UB! Child Write to Disabled.
}
