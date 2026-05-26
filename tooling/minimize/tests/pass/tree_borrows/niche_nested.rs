//@ compile-flags: --minirust-tree-borrows

// Nested niche-bearing enums: `Option<Option<bool>>` produces *two* niche
// claims (one for the outer `Some`, one for the inner `Some`) on the same
// `T_n`'s ancestor chain. Mutating the inner bool to another valid value
// keeps both claims satisfied; no UB.

fn main() {
    let mut x: Option<Option<bool>> = Some(Some(true));
    let inner = match &mut x {
        Some(s) => match s {
            Some(b) => b,
            None => unreachable!(),
        },
        None => unreachable!(),
    };
    *inner = false;
    assert!(matches!(x, Some(Some(false))));
}
