//@ compile-flags: --minimize-tree-borrows

fn main() {
    let arr = [0,1,2,3,4];
    // Since we have a shared reference to a singleton array, and there are no
    // UnsafeCells in the pointee, the surrounding bytes will have the Frozen permission.
    let elem = &arr[1];
    let ptr: *mut u32 = elem as *const _ as *mut _;
    unsafe {
        *ptr.add(1) = 10; // UB! Child write to Frozen
    }
}
