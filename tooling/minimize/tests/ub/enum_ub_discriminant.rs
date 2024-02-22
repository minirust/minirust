
extern crate intrinsics;
use intrinsics::*;

fn print_opt_bool(b: Option<bool>) {
    match b {
        None => print(-1),
        Some(false) => print(0),
        Some(true) => print(1),
    }
}

fn main() {
    unsafe {
        let x = 12u8;
        let x_ptr: *const u8 = &x;
        print_opt_bool(*(x_ptr as *const Option<bool>));
    }
}
