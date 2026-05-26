//@ compile-flags: --minirust-tree-borrows

// §4.6 of the proposal: a sibling Foreign Write that overwrites the niche
// with a legitimate value (`None` here) is not UB. The niche check fires on
// the *pre-write* bytes (still encoding `Some(true)`), passes, and
// discharges the claim. The subsequent discriminant read decodes the new
// `None`-encoded bytes without firing the check.

fn main() {
    let mut x: Option<bool> = Some(true);
    // Hold an `&mut` to the inner bool -- establishes T_n with a `Some` claim.
    let _inner = match &mut x {
        Some(b) => b,
        None => unreachable!(),
    };
    // Sibling Foreign Write through a raw pointer to `x`: discharges the
    // claim after a passing pre-write decode (`Some(true)`).
    unsafe { *(&raw mut x) = None; }
    // No UB: claim was discharged, discriminant read decodes to `None`.
    assert!(x.is_none());
}
