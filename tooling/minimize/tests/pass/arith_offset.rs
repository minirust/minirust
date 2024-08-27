//@revisions: basic tree
//@[tree]compile-flags: --minimize-tree-borrows

fn wrapping_offset() {
    let data = [1u8, 2, 3, 4, 5];
    let first = &data[0] as *const u8;

    unsafe {
        assert!(*first == 1);
        assert!(*first.wrapping_offset(2) == 3);
        assert!(*first.wrapping_offset(4) == 5);
        assert!(*first.wrapping_offset(42).wrapping_offset(-42) == 1);
        assert!(*first.wrapping_offset(-42).wrapping_offset(42) == 1);
    }

    let last = &data[4] as *const u8;

    unsafe {
        assert!(*last == 5);
        assert!(*last.wrapping_offset(-2) == 3);
        assert!(*last.wrapping_offset(-4) == 1);
        assert!(*last.wrapping_offset(42).wrapping_offset(-42) == 5);
        assert!(*last.wrapping_offset(-42).wrapping_offset(42) == 5);
    }
}

fn wrapping_add() {
    let data = [1u8, 2, 3, 4, 5];
    let first = &data[0] as *const u8;

    unsafe {
        assert!(*first == 1);
        assert!(*first.wrapping_add(2) == 3);
        assert!(*first.wrapping_add(4) == 5);
        assert!(*first.wrapping_add(42).wrapping_sub(42) == 1);
    }
}

fn wrapping_sub() {
    let data = [1u8, 2, 3, 4, 5];
    let last = &data[4] as *const u8;

    unsafe {
        assert!(*last == 5);
        assert!(*last.wrapping_sub(2) == 3);
        assert!(*last.wrapping_sub(4) == 1);
        assert!(*last.wrapping_sub(42).wrapping_add(42) == 5);
    }
}


fn main() {
    wrapping_offset();
    wrapping_add();
    wrapping_sub();
}
