extern crate intrinsics;
use intrinsics::*;

fn thread() {
    print(1);
}

fn main() {
    // FnDef is ZeroSized. Not a function pointer.
    // We can use this
    let x = thread as fn();
    let thread_id = spawn(x);
    join(thread_id);
}
