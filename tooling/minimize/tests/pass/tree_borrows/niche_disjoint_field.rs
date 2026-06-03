//@ compile-flags: --minirust-tree-borrows

// §4.10c of the proposal: binding a field that is *disjoint* from the
// enum's niche region. The retag's disjoint clause kicks in -- no `T_n` is
// created -- so the rebind behaves as a plain TB reborrow. Mutating the
// bound field freely is fine; any niche violation would have to come from
// subobject-provenance violations, which are out of scope for this test.

#![allow(dead_code)]
#[repr(C)]
struct Pair(u32, bool);

enum E {
    Has(Pair),
    Nothing,
}

fn main() {
    let mut x: E = E::Has(Pair(42, true));
    // Bind only the u32 -- the niche region (the bool byte) is disjoint.
    // No `T_n` should be attached; the retag is the plain single-tag one.
    let first = match &mut x {
        E::Has(Pair(a, _)) => a,
        _ => unreachable!(),
    };
    *first = 99;
    assert!(matches!(x, E::Has(Pair(99, true))));
}
