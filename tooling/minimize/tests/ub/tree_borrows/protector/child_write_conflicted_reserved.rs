//@ compile-flags: --minimize-tree-borrows

// Check that a foreign read makes future child writes to this mutable reference UB.

fn foo(x: *mut i32, y: &mut i32) {
    // (x, Reserved) -> (y, P[Reserved (conflicted: false)])
    unsafe { assert!(*x == 31) }; // (x, Reserved) -> (y, P[Reserved (conflicted: true)])
    *y = 42; // UB! Child Write to Protected Conflicted Reserved.
}

fn main() {
    let x = &mut 31; // (x, Reserved)   
    let xraw = x as *mut i32; // (x, Reserved) 
    let y = unsafe { &mut *xraw }; // (x, Reserved) -> (y, Reserved)

    foo(xraw, y);
}
