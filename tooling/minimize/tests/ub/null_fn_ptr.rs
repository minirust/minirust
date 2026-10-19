use std::mem::transmute;

fn main() {
    unsafe {
        #[allow(invalid_value)]
        let _x = transmute::<usize, fn()>(0);
    }
}
