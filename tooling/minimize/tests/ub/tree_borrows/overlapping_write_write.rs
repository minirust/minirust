//@ compile-flags: --minimize-tree-borrows
fn main() {
    let parent = &mut 31; // (parent, Reserved)   
    let x = parent as *mut i32; // (parent, Reserved) 
    let y = unsafe { &mut *x }; // (parent, Reserved) -> (y, Reserved)
    *y = 42;

    unsafe { 
        *x = 57; // (parent, Reserved) -> (y, Disabled)
        assert!(*x == 57); // (parent, Reserved) -> (y, Disabled)
    } 

    *y = 31; // UB! Child Write to Disabled 
}
