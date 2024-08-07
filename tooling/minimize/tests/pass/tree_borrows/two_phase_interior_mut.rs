//@ compile-flags: --minimize-tree-borrows

// This test was taken from Miri Tree Borrows
// https://github.com/rust-lang/miri/blob/master/tests/pass/tree_borrows/2phase-interiormut.rs

use core::cell::Cell;

trait Thing: Sized {
    fn do_the_thing(&mut self, _s: i32) {}
}
impl<T> Thing for Cell<T> {}

fn main() {
    let mut x = Cell::new(1);
    let l = &x;

    x.do_the_thing({
        // Several Foreign accesses (both Reads and Writes) to the location
        // being reborrowed. Reserved + unprotected + interior mut
        // makes the pointer immune to everything as long as all accesses
        // are child accesses to its parent pointer x.
        x.set(3);
        l.set(4);
        x.get() + l.get()
    });
}
