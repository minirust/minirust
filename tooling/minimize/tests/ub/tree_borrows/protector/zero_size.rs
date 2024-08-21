//@ compile-flags: --minimize-tree-borrows

// Check that protectors work even with a zero-sized pointee.

fn foo(x: *mut i32, y: &mut()) {
    // (x, Reserved) -> (y, P[Reserved])
    let y = y as *mut () as *mut i32;
    unsafe { *y = 42 }; // y becomes Accessed and Active.
    unsafe { assert!(*x == 42) }; // UB! Foreign Write to Protected Active.
    
}

fn main() {
    let x = &mut 31; // (x, Reserved)   
    let xraw = x as *mut i32; // (x, Reserved)
    let y = unsafe { &mut *(xraw as *mut ()) }; // (x, Reserved) -> (y, Reserved)

    foo(xraw, y);
}
