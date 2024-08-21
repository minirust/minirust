//@ compile-flags: --minimize-tree-borrows

// Check that a foreign write of a protected mutable reference with frozen permission is UB.

fn foo(x: *mut i32, _frozen: &i32) {
    // (x, Reserved) -> (y, Reserved) -> (_frozen, P[Frozen])
    unsafe {
        *x = 42; // UB! Foreign Write to Protected Frozen.
    } 
}

fn main() {
    let x = &mut 31; // (x, Reserved)   
    let xraw = x as *mut i32; // (x, Reserved) 
    let y = unsafe { &mut *xraw }; // (x, Reserved) -> (y, Reserved)

    foo(xraw, y);
}
