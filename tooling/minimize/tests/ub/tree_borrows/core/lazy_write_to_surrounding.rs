//@ compile-flags: --minimize-tree-borrows

fn main() {
    let arr = [0,1,2,3,4];
    // Since we have a shared reference to a singleton array, and there are no
    // UnsafeCells in the pointee, the surrounding bytes will have the Frozen permission.
    let sub_arr = &arr[1..2];
    let ptr: *mut u32 = sub_arr.as_ptr() as *mut _;
    unsafe {
        *ptr.add(1) = 10; // UB! Child write to Frozen
    }
}
