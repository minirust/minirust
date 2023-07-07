//! When used by `minimize`, a call to these functions will be replaced by a `CallIntrinsic`.
//! The bodies of these functions are mostly used through `tests/rust.sh`.

#![feature(allocator_api)]

use std::fmt::Display;
use std::alloc::{System, Layout, Allocator};
use std::mem;
use std::ptr::NonNull;
use std::sync::Mutex;
use std::thread::{JoinHandle, self};

static THREAD_HANDLES: Mutex<Vec<Option<JoinHandle<()>>>> = Mutex::new(Vec::new());

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

// Spawn returns the index+1 of the thread because we don't have a JoinHandle for thread 0.
pub fn spawn<F>(f: F) -> usize
where F: FnOnce() -> () + Send + 'static,
{
    let handle = thread::spawn(f);
    let mut vec = THREAD_HANDLES.lock().unwrap();
    vec.push(Some(handle));
    vec.len()
}

pub fn join(index: usize) {
    if index == 0 { panic!("Can not join thread 0.") }

    let mut vec = THREAD_HANDLES.lock().unwrap();
    let join_handle = vec.get_mut(index-1);

    if let Some(join_handle) = join_handle {
        let join_handle = mem::replace(join_handle, None);

        drop(vec);

        if let Some(join_handle) = join_handle {
            join_handle.join().unwrap();
        }

        else {
            panic!("Joining thread that is joined by other thread.")
        }
    }
    else {
        panic!("Joining non existent thread.")
    }
}
