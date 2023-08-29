extern crate intrinsics;
use intrinsics::*;

extern "C" fn thread(_: *const ()) {
    print(1);
}

fn main() {
    let data_ptr = &() as *const ();
    
    let fn_ptr = thread as extern "C" fn(*const ());
    let thread_id = spawn(fn_ptr, data_ptr);
    join(thread_id);
}
