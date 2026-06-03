//@ compile-flags: --minirust-tree-borrows

// Inverse of §4.6: a sibling Foreign Write whose *pre-write* bytes have
// already been corrupted (via a preceding Local-write-through-T_c bypass).
// The niche check fires before the write commits -- and fails -- because the
// bytes already encode `None`, not the claimed `Some`. UB at the foreign
// write itself, before its bytes land in memory.

fn main() {
    let mut x: Option<bool> = Some(true);
    let inner = match &mut x {
        Some(b) => b,
        None => unreachable!(),
    };
    // Step 1: Local Write -- corrupts the niche silently (Local for T_n).
    unsafe { *(inner as *mut bool as *mut u8) = 2; }
    // Step 2: Foreign Write through `&raw mut x`. Niche check fires on
    // pre-write bytes (current = 2 = None) -- fails to match the claimed
    // `Some` → UB at this write.
    unsafe { *(&raw mut x) = None; }
}
