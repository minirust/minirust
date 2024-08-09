//@ compile-flags: --minimize-tree-borrows

// Check that foreign writing to an actively protected mutable reference with frozen permission is UB

fn foo(x: *mut i32, _y: &i32) {
    // (x, P[Reserved]) -> (y, P[Reserved])
    unsafe {
        assert!(*x == 31); // (x, P[Active]) -> (y, P[Frozen])
        *x = 42; // UB! Foreign Write to Protected Frozen.
    } 
}

fn main() {
    let x = &mut 31; // (x, Reserved)   
    let xraw = x as *mut i32; // (x, Reserved) 
    let y = unsafe { &mut *xraw }; // (x, Reserved) -> (y, Reserved)

    foo(xraw, y);
}
