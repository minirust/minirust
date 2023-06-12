//! When used by `minimize`, a call to these functions will be replaced by a `CallIntrinsic`.
//! The bodies of these functions are mostly used through `tests/rust.sh`.

#![feature(allocator_api)]

use std::fmt::Display;
use std::alloc::{System, Layout, Allocator};
use std::ptr::NonNull;

pub fn print(t: impl Display) {
    println!("{t}");
}

pub fn eprint(t: impl Display) {
    eprintln!("{t}");
}

pub fn exit() {
    std::process::exit(0);
}

pub unsafe fn allocate(size: usize, align: usize) -> *mut u8 {
    let layout = Layout::from_size_align(size, align).unwrap();
    System.allocate(layout).unwrap().as_ptr() as *mut u8
}

pub unsafe fn deallocate(ptr: *mut u8, size: usize, align: usize) {
    let ptr = NonNull::new(ptr).unwrap();
    let layout = Layout::from_size_align(size, align).unwrap();
    unsafe { System.deallocate(ptr, layout); }
}
