//! Ensures an out-of-bounds index operation without an inbounds-check is UB.

fn main() {
    let x = [1, 2, 3_u8];
    let y: &[u8] = &x[..2];
    // UB: the index is out-of-bounds even though it is still in the allocation of x.
    // This generates an assume condition, which is then caught by MiniRust
    let _z = unsafe { y.get_unchecked(2) };
}
