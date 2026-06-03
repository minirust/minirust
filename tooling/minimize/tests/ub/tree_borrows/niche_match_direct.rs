//@ compile-flags: --minirust-tree-borrows

// §4.9 of the proposal: matching `x` directly with a `ref mut` binding
// (no intermediate `&mut x` reborrow). The tree shape is `T_x → T_n → T_c`
// instead of `T_x → T_match → T_n → T_c`. The niche check still fires on a
// Foreign access from a sibling of `T_n` (here: a fresh reborrow of `x`).
// UB is the same as §4.1.

fn main() {
    let mut x: Option<bool> = Some(true);
    // Take T_c via a `ref mut` binding directly on x (no intermediate
    // `&mut x`). Convert to a raw pointer immediately so the borrow ends.
    let p = match x {
        Some(ref mut b) => b as *mut bool as *mut u8,
        None => unreachable!(),
    };
    unsafe {
        *p = 2;
    } // Local Write -- breaks niche silently.
    let _ = matches!(x, Some(_));
}
