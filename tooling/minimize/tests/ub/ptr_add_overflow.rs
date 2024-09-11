fn main() {
    let x = &[0i32; 2];
    let x = std::ptr::from_ref(&x[0]).wrapping_add(1);
    // Will be equal to -4isize when multiplied be the size (4) -- and that step does not itself overflow.
    let offset = !0usize >> 2;
    // However, the usize-to-isize cast is lossy and hence this should be UB.
    // Or put differently, -4isize as usize is out-of-bounds.
    unsafe { x.add(offset).read() };
}
