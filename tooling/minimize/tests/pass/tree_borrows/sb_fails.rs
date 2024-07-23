//@ compile-flags: --minimize-tree-borrows

// The tests were taken from Miri Tree Borrows 
// https://github.com/rust-lang/miri/blob/6680b2fa09496b60c40c6ce09449f46efbf253d5/tests/pass/tree_borrows/sb_fails.rs
// FIXME: Some tests in the original test suite are not currently supported by MiniRust 

mod fnentry_invalidation {
    // Copied directly from fail/stacked_borrows/fnentry_invalidation.rs
    // Version that fails TB: fail/tree_borrows/fnentry_invalidation.rs
    pub fn main() {
        let mut x = 0i32;
        let z = &mut x as *mut i32;
        x.do_bad();
        unsafe {
            let _oof = *z;
            // In SB this is an error, but in TB the mutable reborrow did
            // not invalidate z for reading.
        }
    }

    trait Bad {
        fn do_bad(&mut self) {
            // who knows
        }
    }

    impl Bad for i32 {}
}

mod pass_invalid_mut {
    // Copied directly from fail/stacked_borrows/pass_invalid_mut.rs
    // Version that fails TB: fail/tree_borrows/pass_invalid_mut.rs
    fn foo(_: &mut i32) {}

    pub fn main() {
        let x = &mut 42;
        let xraw = x as *mut _;
        let xref = unsafe { &mut *xraw };
        let _val = unsafe { *xraw }; // In SB this invalidates xref...
        foo(xref); // ...which then cannot be reborrowed here.
        // But in TB xref is Reserved and thus still writeable.
    }
}

mod return_invalid_mut {
    // Copied directly from fail/stacked_borrows/return_invalid_mut.rs
    // Version that fails TB: fail/tree_borrows/return_invalid_mut.rs
    fn foo(x: &mut (i32, i32)) -> &mut i32 {
        let xraw = x as *mut (i32, i32);
        let ret = unsafe { &mut (*xraw).1 };
        let _val = unsafe { *xraw }; // In SB this invalidates ret...
        ret // ...which then cannot be reborrowed here.
        // But in TB ret is Reserved and thus still writeable.
    }

    pub fn main() {
        foo(&mut (1, 2));
    }
}

fn main() {
    fnentry_invalidation::main();
    pass_invalid_mut::main();
    return_invalid_mut::main();
}