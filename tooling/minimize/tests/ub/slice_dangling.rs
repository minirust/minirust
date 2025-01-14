//! Ensures that the entire slice needs to be dereferencable.

fn main() {
    let x = [1, 2_u32];
    // UB: the allocation of x is only 2 elements large, not 3.
    let _z = unsafe { core::slice::from_raw_parts::<'_, u32>((&x).as_ptr(), 3) };
}
