//@ compile-flags: --minimize-tree-borrows

// Check that foreign reading from an actively protected mutable reference with active permission is UB

fn foo(x: *mut i32, y: &mut i32) {
    // (x, P[Reserved]) -> (y, P[Reserved])
    *y = 57; // (x, P[Active]) -> (y, P[Active])
    unsafe { assert!(*x == 57); } // UB! Foreign Read from Protected Active.
}

fn main() {
    let x = &mut 31; // (x, Reserved)   
    let xraw = x as *mut i32; // (x, Reserved) 
    let y = unsafe { &mut *xraw }; // (x, Reserved) -> (y, Reserved)

    foo(xraw, y);
}
