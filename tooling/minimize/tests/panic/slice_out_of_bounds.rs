//! Ensures that a normal index operation panics for out of bounds indexes.

#[allow(unconditional_panic)]
fn main() {
    let x = [1, 2_u32];
    let x: &[u32] = &x;
    // Assertion failure in in-bounds check:
    let _y = x[2];
}
