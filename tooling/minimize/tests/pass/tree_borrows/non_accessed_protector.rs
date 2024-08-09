//@ compile-flags: --minimize-tree-borrows

// Check that non-accessed protected nodes do not cause UB on foreign writes.

fn foo(x: *mut u8, _y: &mut u8) {
    // (x, P[[R, R]]) -> (y, P[[R, R]])
    unsafe { *x.add(1) = 57; } // Okay to Foreign Write to y
}

fn main() {
    let mut data = [42u8, 57]; 
    let x = &mut data[0] as *mut u8; // (x, [R, R]) 
    let y = unsafe { &mut *x }; // (x, [R, R]) -> (y, [R, R])
    
    foo(x, y);
}
