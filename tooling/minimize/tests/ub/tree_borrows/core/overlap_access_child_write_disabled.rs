//@ compile-flags: --minimize-tree-borrows

// Check that writing to a location via a mutable reference can also
// make writing to this location via another mutable reference UB.

fn main() {
    unsafe {
        let mut data = [42u8, 57]; 
        let x = &mut data[0] as *mut u8; // (x, [R, R]) 
        let y = &mut data[1] as *mut u8; // (x, [R, R]) (y, [R, R]) 
        // Using x disables y at data[0].
        *x = 42; // (x, [A, R]) (y, [D, R]) 
        // Using y disables x at data[1].
        *y = 57; // (x, [A, D]) (y, [D, A])

        // UB! The write to y has disabled x at data[1].
        *x.add(1) = 57;
    }
}
