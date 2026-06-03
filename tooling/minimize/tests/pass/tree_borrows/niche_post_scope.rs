//@ compile-flags: --minirust-tree-borrows

// §4.8 of the proposal: after the niche-bearing binding has fallen out of
// scope, reassigning `x` to a different variant must not be UB. The
// reassignment is a Foreign Write through `T_x` that touches the niche
// region; the niche check fires on the pre-write bytes (still encoding
// `Some`) -- passes -- and *discharges* the claim. The subsequent
// discriminant read sees `None` without firing any niche check.

fn main() {
    let mut x: Option<bool> = Some(true);
    {
        let inner = match &mut x {
            Some(b) => b,
            None => unreachable!(),
        };
        *inner = false;
    }
    // `inner` is out of scope; reassign x as a different variant. This is a
    // Foreign Write for T_n, which discharges the claim on a passing decode.
    x = None;
    assert!(x.is_none());
}
