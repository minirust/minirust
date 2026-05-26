//@ compile-flags: --minirust-tree-borrows

// Combination of §4.12 + the dealloc Foreign Write: a Local Read through
// `T_c` after a niche-breaking Local Write fires no niche check (Local for
// `T_n`). The corrupted bytes survive untouched until function exit, at
// which point `storage_dead` deallocates the local -- a Foreign Write that
// fires the niche check on `T_n`'s claim against the still-corrupted bytes
// → UB at the dealloc.

fn main() {
    let mut x: Option<bool> = Some(true);
    let p = match &mut x {
        Some(b) => b as *mut bool as *mut u8,
        None => unreachable!(),
    };
    // Local Write through T_c -- silently breaks the niche (no check).
    unsafe {
        *p = 2;
    }
    // Local Read through T_c -- also Local for T_n, no check fires.
    // The user observes the corrupted byte without UB.
    let b: u8 = unsafe { *p };
    assert!(b == 2);
    // No restoration: function returns with the niche broken. The
    // implicit `storage_dead` of `x` is a Foreign Write through the
    // deallocator's tag, which fires the niche check and UBs.
}
