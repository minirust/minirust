//@revisions: basic tree
//@[tree]compile-flags: --minimize-tree-borrows
#![feature(ptr_sub_ptr)]

use std::ptr;

fn main() {
    ptr();
    ptr_mut();
    ptr_compare();
    ptr2ptr();
    offset();
    add();
    wrapping_offset();
    wrapping_add();
    wrapping_sub();
    offset_from();
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

fn offset() {
    let data = [1u16, 2, 3, 4, 5];
    let first = &data[0] as *const u16;
    unsafe {
        let ptr = first.offset(2);
        assert!(*ptr == 3);
        let ptr = ptr.offset(-1);
        assert!(*ptr == 2);
    }
}

fn add() {
    let data = [1u16, 2, 3, 4, 5];
    let first = &data[0] as *const u16;
    unsafe {
        let ptr = first.add(2);
        assert!(*ptr == 3);
    }
}

fn wrapping_offset() {
    let data = [1i32, 2, 3, 4, 5];
    let first = &data[0] as *const i32;

    unsafe {
        assert!(*first == 1);
        assert!(*first.wrapping_offset(2) == 3);
        assert!(*first.wrapping_offset(4) == 5);
        assert!(*first.wrapping_offset(42).wrapping_offset(-42) == 1);
        assert!(*first.wrapping_offset(-42).wrapping_offset(42) == 1);
    }

    let last = &data[4] as *const i32;

    unsafe {
        assert!(*last == 5);
        assert!(*last.wrapping_offset(-2) == 3);
        assert!(*last.wrapping_offset(-4) == 1);
        assert!(*last.wrapping_offset(42).wrapping_offset(-42) == 5);
        assert!(*last.wrapping_offset(-42).wrapping_offset(42) == 5);
    }
}

fn wrapping_add() {
    let data = [1u64, 2, 3, 4, 5];
    let first = &data[0] as *const u64;

    unsafe {
        assert!(*first == 1);
        assert!(*first.wrapping_add(2) == 3);
        assert!(*first.wrapping_add(4) == 5);
        assert!(*first.wrapping_add(42).wrapping_sub(42) == 1);
    }
}

fn wrapping_sub() {
    let data = [1u64, 2, 3, 4, 5];
    let last = &data[4] as *const u64;

    unsafe {
        assert!(*last == 5);
        assert!(*last.wrapping_sub(2) == 3);
        assert!(*last.wrapping_sub(4) == 1);
        assert!(*last.wrapping_sub(42).wrapping_add(42) == 5);
    }
}

fn offset_from() {
    let data = [1u16, 2, 3, 4, 5];
    unsafe {
        assert!(ptr::from_ref(&data[4]).offset_from(&data[0]) == 4);
        assert!(ptr::from_ref(&data[0]).offset_from(&data[4]) == -4);
        assert!(ptr::from_ref(&data[4]).sub_ptr(&data[0]) == 4);
    }
}
