#[allow(unconditional_panic)]
fn main() {
    let x = [1, 2];
    // Assertion failure in in-bounds check:
    let _y = x[2];
}
