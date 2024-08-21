//@ compile-flags: --minimize-tree-borrows

// This test was taken from Miri Tree Borrows
// https://github.com/rust-lang/miri/blob/master/tests/pass/tree_borrows/end-of-protector.rs


// Check that a protector goes back to normal behavior when the function
// returns.

fn main() {
    unsafe {
        let data = &mut 0u8;
        let x = &mut *data;
        assert!(*x == 0);
        do_nothing(x); // creates then removes a Protector for a child of x
        let y = &mut *data;
        *y = 1;
        assert!(*y == 1);
    }
}

unsafe fn do_nothing(x: &mut u8) {
    assert!(*x == 0);
}
