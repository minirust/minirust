//@ compile-flags: --minirust-tree-borrows

// §4.1 of the niche-extension proposal (the canonical OP example, adapted to
// `Option<bool>` since `Option<NonZeroU32>` does not lower in minimize).
// Take an `&mut` to the niche-bearing field, raw-cast it, and overwrite the
// bytes with a value outside the variant's valid range. A later discriminant
// read through `x` is Foreign to `T_n` and touches the niche bytes, so the
// niche check fires and UB occurs at that read.

fn main() {
    let mut x: Option<bool> = Some(true);
    let inner = match &mut x {
        Some(b) => b,
        None => unreachable!(),
    };
    // Local Write through T_c -- corrupts the niche.
    unsafe { *(inner as *mut bool as *mut u8) = 2; }
    // Foreign Read through a fresh reborrow of x -- fires the niche check.
    let _ = match &x {
        Some(_) => 1,
        None => 0,
    };
}
