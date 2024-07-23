//@ compile-flags: --minimize-tree-borrows
fn main() {
    let x = &mut 31; // (x, Reserved)   
    let xraw = x as *mut i32; // (x, Reserved) 
    let y = unsafe { &mut *xraw }; // (x, Reserved) -> (y, Reserved)
    *y = 42; // (x, Active) -> (y, Active)
    assert!(unsafe { *xraw } == 42); // (x, Active) -> (y, Frozen)
    *y = 31; // UB! Child Write to Frozen 
}
