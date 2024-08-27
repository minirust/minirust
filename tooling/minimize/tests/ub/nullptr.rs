use std::mem::transmute;

fn main() {
    unsafe {
        let _x = *transmute::<usize, *const i32>(0);
    }
}
