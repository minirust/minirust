extern crate intrinsics;
use intrinsics::*;

extern "C" fn thread(data_ptr: *const ()) {
    let lock_id = unsafe { *(data_ptr as *const usize) };
    acquire(lock_id);
    print(1);
    release(lock_id);
}

fn main() {
    let lock_id = create_lock();
    let data_ptr = &lock_id as *const usize as *const ();
    let fn_ptr = thread as extern "C" fn(*const ());

    acquire(lock_id);
    let thread_id = spawn(fn_ptr, data_ptr);
    print(0);
    release(lock_id);
    join(thread_id);
}
