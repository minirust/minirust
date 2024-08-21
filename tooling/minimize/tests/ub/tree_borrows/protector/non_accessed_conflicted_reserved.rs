//@ compile-flags: --minimize-tree-borrows

// Check that a foreign read makes future child writes to this mutable reference UB, even though the reference is non-accessed.

fn foo(x: *mut u8, y: &mut u8) {
    // (x, [R, R]) -> (y, P[[R, R]])
    let yraw = y as *mut u8;
    unsafe {
        assert!(*x.add(1) == 57); // (x, [R, R]) -> (y, P[[R, RC]])
        assert!(*yraw.add(1) == 57); // y[1] becomes Accessed (Actively protected)
        *yraw.add(1) = 42; // UB! Child Write to Actively Protected Conflicted Reserved.
    } 
}

fn main() {
    let mut data = [42u8, 57]; 
    let x = &mut data[0] as *mut u8; // (x, [R, R]) 
    let y = unsafe { &mut *x }; // (x, [R, R]) -> (y, [R, R])
    foo(x, y);
}
