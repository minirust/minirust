//@ compile-flags: --minimize-tree-borrows

// The tests were taken from Miri Tree Borrows 
// https://github.com/rust-lang/miri/blob/master/tests/pass/tree_borrows/sb_fails.rs

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

mod static_memory_modification {
    // Copied directly from fail/stacked_borrows/static_memory_modification.rs
    // Version that fails TB: fail/tree_borrows/static_memory_modification.rs
    static X: usize = 5;

    #[allow(mutable_transmutes)]
    pub fn main() {
        let x = unsafe {
            std::mem::transmute::<&usize, &mut usize>(&X) // In SB this mutable reborrow fails.
            // But in TB we are allowed to transmute as long as we don't write.
        };
        assert!(*&*x == 5);
    }
}

#[allow(unused_assignments)] // spurious warning
fn interior_mut_reborrow() {
    use std::cell::UnsafeCell;

    let mut c = UnsafeCell::new(42);
    let ptr = c.get(); // first create interior mutable ptr
    c = UnsafeCell::new(13); // then write to parent
    unsafe { assert!( *ptr == 13); } // then read through previous ptr
}

fn main() {
    fnentry_invalidation::main();
    pass_invalid_mut::main();
    return_invalid_mut::main();
    static_memory_modification::main();
    interior_mut_reborrow();
}
