//@ compile-flags: --minirust-tree-borrows

// §4.2 variant -- the "would-have-been-restored" case. A Foreign Read
// *between* the breaking Local Write and the restoring Local Write fires
// the niche check on the *current* (broken) bytes. UB at the intermediate
// read; the restoration never happens.

fn main() {
    let mut x: Option<bool> = Some(true);
    let inner = match &mut x {
        Some(b) => b,
        None => unreachable!(),
    };
    let p = inner as *mut bool as *mut u8;
    unsafe {
        *p = 2; // break niche -- Local for T_n, no check fires
        // Foreign Read fires the niche check on current bytes (2 = None) → UB.
        let _ = matches!(x, Some(_));
        *p = 0; // would restore -- unreached
    }
}
