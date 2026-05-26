//@ compile-flags: --minirust-tree-borrows

// Deallocation fires a Foreign Write across the whole allocation through
// the deallocator's tag, foreign for every `T_n` in the tree. If a claim's
// niche bytes have been corrupted by a Local-write-through-T_c bypass and
// not restored, the dealloc's niche check decodes the broken bytes and
// UBs -- even though there was no explicit user-level discriminant read.

fn run() {
    let mut x: Option<bool> = Some(true);
    let inner = match &mut x {
        Some(b) => b,
        None => unreachable!(),
    };
    // Break niche through T_c; no Foreign access yet, so no UB.
    unsafe { *(inner as *mut bool as *mut u8) = 2; }
    // Function returns: x's storage is freed. The dealloc's Foreign Write
    // fires the niche check on T_n's claim against the corrupted bytes →
    // UB at the dealloc.
}

fn main() {
    run();
}
