fn main() {
    let x = &[0u8; 2];
    let x = std::ptr::from_ref(&x[0]).wrapping_add(1);
    // If the `!0` is interpreted as `isize`, it is just `-1` and hence harmless.
    // However, this is unsigned arithmetic, so really this is `usize::MAX` and hence UB.
    unsafe { x.add(!0).read() };
}
