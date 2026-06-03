//@ compile-flags: --minirust-tree-borrows

// §4.12 of the proposal (simplified): a Read through `T_c` (or any
// descendant of `T_n`) is *Local* for `T_n` -- so the niche check does
// not fire, and the read returns whatever the bytes encode without
// asserting the claim. This is the load-bearing case for letting users
// observe a transiently broken niche through their own `T_c` reborrow
// without UB; the discriminant-elimination theorem is about
// observations through `T_x`, not through `T_c`.
//
// We restore the niche before returning so that dealloc's own Foreign
// Write doesn't fire UB on its niche check.

fn main() {
    let mut x: Option<bool> = Some(true);
    let p = match &mut x {
        Some(b) => b as *mut bool as *mut u8,
        None => unreachable!(),
    };
    // Local Write through T_c -- silently breaks the niche.
    unsafe { *p = 2; }
    // Local Read through T_c -- Local for T_n, niche check does not fire.
    let b: u8 = unsafe { *p };
    assert!(b == 2);
    // Restore the niche so dealloc's foreign write doesn't UB.
    unsafe { *p = 0; }
}
