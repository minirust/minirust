//! When used by `minimize`, a call to these functions will be replaced by a `CallIntrinsic`.
//! The bodies of these functions are mostly used through `tests/rust.sh`.

#![feature(allocator_api)]
#![feature(atomic_from_ptr)]

use std::fmt::Display;
use std::alloc::{System, Layout, Allocator};
use std::ptr::NonNull;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread::{JoinHandle, self};

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


static JOIN_HANDLES: Mutex<Vec<Option<JoinHandle<()>>>> = Mutex::new( Vec::new() );

struct SendPtr<T>(*const T);
unsafe impl<T> Send for SendPtr<T> {}

pub fn spawn(fn_ptr: extern "C" fn(*const ()), data_ptr: *const ()) -> usize {
    let mut join_handles = JOIN_HANDLES.lock().unwrap();

    if join_handles.len() == 0 {
        join_handles.push(None);
    }

    let ptr = SendPtr(data_ptr);
    let handle = thread::spawn(
        move || {
            let ptr = ptr; // avoid per-field closure capturing
            fn_ptr(ptr.0);
        }
    );
    join_handles.push(Some(handle));

    // Return the index of the element we just pushed.
    join_handles.len()-1
}

pub fn join(thread_id: usize) {
    let mut join_handles = JOIN_HANDLES.lock().unwrap();
    let handle = join_handles[thread_id].take().unwrap();
    handle.join().unwrap();
}


#[derive(PartialEq)]
enum LockState {
    Open,
    Locked,
}
static LOCKS: Mutex<Vec<LockState>> = Mutex::new( Vec::new() );

pub fn create_lock() -> usize {
    let mut locks = LOCKS.lock().unwrap();

    let id = locks.len();
    locks.push(LockState::Open);
    id
}

// Spin (with parking) until the lock is open.
pub fn acquire(lock_id: usize) {
    loop {
        let mut locks = LOCKS.lock().unwrap();

        if locks[lock_id] == LockState::Open {
            // We can grab the lock! Return successfully.
            locks[lock_id] = LockState::Locked;

            return;
        }

        drop(locks);
        thread::park()
    }
}

// Unparks all threads for simplicity.
pub fn release(lock_id: usize) {
    LOCKS.lock().unwrap()[lock_id] = LockState::Open;

    let join_handles = JOIN_HANDLES.lock().unwrap();
    // We don't precisely track who is waiting for which lock, so
    // we just wake up *all* the threads.
    for handle in join_handles.iter().flatten() {
        handle.thread().unpark();
    }
}


pub unsafe fn atomic_store(ptr: *mut u32, value: u32) {
    let atomic = AtomicU32::from_ptr(ptr);
    atomic.store(value, Ordering::SeqCst);
}

pub unsafe fn atomic_load(ptr: *mut u32) -> u32 {
    let atomic = AtomicU32::from_ptr(ptr);
    atomic.load(Ordering::SeqCst)
}

pub unsafe fn compare_exchange(ptr: *mut u32, current: u32, new: u32) -> u32 {
    let atomic = AtomicU32::from_ptr(ptr);
    let res = atomic.compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst);
    match res {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}
