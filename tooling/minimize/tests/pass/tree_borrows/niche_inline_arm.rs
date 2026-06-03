//@ compile-flags: --minirust-tree-borrows

// §4.7 of the proposal: an inline arm body that mutates the niche-bearing
// field to another valid value, then observes the discriminant outside the
// match. The discriminant-elimination optimisation applies -- the assert is
// statically `true`. No UB fires because the only writes are Local through
// `T_c`, and the post-match Foreign Read decodes valid `Some` bytes.

fn main() {
    let mut x: Option<bool> = Some(true);
    match &mut x {
        Some(b) => {
            *b = false;
        }
        None => unreachable!(),
    }
    assert!(matches!(x, Some(_)));
}
