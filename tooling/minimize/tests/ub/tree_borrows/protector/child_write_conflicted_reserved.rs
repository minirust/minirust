//@ compile-flags: --minimize-tree-borrows

// Check that a foreign read makes a actively protected reserved mutable reference conflicted.
// After that, writing to this mutable referenc is UB.

fn foo(x: *mut i32, y: &mut i32) {
    // (x, P[Reserved]) -> (y, P[Reserved (conflicted: false)])
    unsafe { assert!(*x == 31); } // (x, P[Reserved]) -> (y, P[Reserved (conflicted: true)])
    *y = 42; // UB! Child Write to Protected Conflicted Reserved.
}

fn main() {
    let x = &mut 31; // (x, Reserved)   
    let xraw = x as *mut i32; // (x, Reserved) 
    let y = unsafe { &mut *xraw }; // (x, Reserved) -> (y, Reserved)

    foo(xraw, y);
}
