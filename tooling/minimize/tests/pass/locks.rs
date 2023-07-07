extern crate intrinsics;
use intrinsics::*;

fn thread() {
    acquire(0);
    print(1);
    release(0);
}

fn main() {
    let x = thread as fn();
    create_lock();
    acquire(0);
    let thread_id = spawn(x);
    print(0);
    release(0);
    join(thread_id);
}
