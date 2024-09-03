//@revisions: basic tree
//@[tree]compile-flags: --minimize-tree-borrows

fn main() {
    ptr();
    ptr_mut();
    ptr_compare();
    ptr2ptr();
    wrapping_offset();
    wrapping_add();
    wrapping_sub();
}

fn ptr() {
    let x = 2;
    let y = &x as *const i32;
    let z = unsafe { *y };
    assert!(z == 2);
}

fn ptr_mut() {
    let mut x = 2;
    let y = &mut x as *mut i32;
    unsafe { *y = 3; }
    assert!(x == 3);
}

fn ptr_compare() {
    let x = 2;
    let y = 2;
    let xptr = &x as *const i32;
    let yptr = &y as *const i32;
    assert!(xptr == xptr);
    assert!(xptr != yptr);
    assert!(xptr < yptr || xptr > yptr);
}

fn ptr2ptr() {
    let x = -1;
    let y = &x as *const i32 as *const u32;
    assert!(unsafe { *y } == u32::MAX);
}

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
