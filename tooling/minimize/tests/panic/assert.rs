#[allow(unconditional_panic)]
fn main() {
    // An assert is put before divisions, so this should panic but not create UB.
    let _ = 42 / 0;
}
