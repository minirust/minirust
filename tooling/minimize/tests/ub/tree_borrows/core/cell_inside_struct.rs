//@ compile-flags: --minimize-tree-borrows

use std::cell::Cell;

struct Foo {
    field1: u32,
    field2: Cell<u32>,
}

pub fn main() {
    let root = Foo { field1: 42, field2: Cell::new(88) };

    unsafe {
        let a = &root;
        let a: *mut Foo = a as *const _ as *mut _;

        // Writing to `field2`, which is interior mutable, should be allowed.
        (*a).field2.set(10);

        // Writing to `field1`, which is frozen, should not be allowed.
        (*a).field1 = 88; // UB! Child write to frozen.
    }
}
