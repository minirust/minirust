//@ compile-flags: --minirust-tree-borrows

// Smoke test for the niche-extension's two-child retag (§3.2 of the
// proposal): binding the niche-bearing field by `&mut` exercises the
// `T_n + T_c` retag path. Mutating to another valid bool keeps the bytes
// niche-compliant, so the post-match Foreign Read decodes successfully and
// no UB fires.

fn main() {
    let mut x: Option<bool> = Some(true);
    let inner = match &mut x {
        Some(b) => b,
        None => unreachable!(),
    };
    *inner = false;
    assert!(matches!(x, Some(false)));
}
