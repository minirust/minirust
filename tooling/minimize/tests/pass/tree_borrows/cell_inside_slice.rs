//@ compile-flags: --minirust-tree-borrows

/// Write to Cell objects inside a slice.

use std::cell::Cell;

fn main() {
    let root = &[(Cell::new(0), 1), (Cell::new(2), 3)][..];
    let x = &root[0];
    x.0.set(42);
    assert!(root[0].0.get() == 42);

    let y = &root[1];
    y.0.set(100);
    assert!(root[1].0.get() == 100);

    let root = &[(Cell::new(0), 1), (Cell::new(2), 3), (Cell::new(4), 5)][..];
    let z = &root[2];
    z.0.set(42);
    assert!(root[2].0.get() == 42);

    let root = &[(Cell::new(0), 1)][..];
    let w = &root[0];
    w.0.set(42);
    assert!(root[0].0.get() == 42);
}
