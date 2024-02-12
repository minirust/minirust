#![feature(never_type)]

fn main() {
    // While this is not going to run it is forcing the minimizer to minimize `!`.
    if false {
        unsafe {
            let x = 0u8;
            let x_ptr: *const u8 = &x;
            let _ = *(x_ptr as *const !);
        }
    };
}
