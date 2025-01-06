fn foo(_x: *const dyn std::fmt::Debug) {}

fn main() {
    // cannot use `std::mem::zeroed`: unsupported Rust intrinsic `write_bytes`.
    foo(unsafe { core::mem::transmute::<[usize; 2], *const dyn std::fmt::Debug>([0_usize; 2]) });
}
