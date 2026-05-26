//@ compile-flags: --minirust-tree-borrows

// §4.2 of the niche-extension proposal: a transient niche violation, restored
// before any Foreign access to the niche bytes, is permitted. Both writes
// through `inner` are Local for `T_n`, so the niche check doesn't fire; the
// later Foreign Read through `&x` checks the *current* bytes, which by then
// encode a valid `Some` again.

fn main() {
    let mut x: Option<bool> = Some(true);
    let inner = match &mut x {
        Some(b) => b,
        None => unreachable!(),
    };
    let p = inner as *mut bool as *mut u8;
    unsafe {
        *p = 2; // niche broken -- but Local for T_n, no check fires
        *p = 0; // niche restored to Some(false)
    }
    // Foreign Read through a reborrow of x fires the niche check on the
    // *current* bytes (Some(false)) -- passes.
    assert!(matches!(x, Some(_)));
}
