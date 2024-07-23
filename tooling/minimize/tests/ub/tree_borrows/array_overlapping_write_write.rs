//@ compile-flags: --minimize-tree-borrows
fn main() {
    unsafe { 
        let mut data = [42u8, 57]; 
        let x = &mut data[0] as *mut u8; // (x, [R, R]) 
        let y = &mut data[1] as *mut u8; // (x, [R, R]) (y, [R, R]) 
        *x = 42; // (x, [A, R]) (y, [D, R]) 
        *y = 57; // (x, [A, D]) (y, [D, A]) 
    
        *x.add(1) = 42; // UB! Child Write to Disabled
    }
}
