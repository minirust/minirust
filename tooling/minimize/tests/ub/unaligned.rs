include!("../helper/transmute.rs");

fn main() {
    unsafe {
        let _i  = *transmute::<usize, *const i32>(1);
    }
}
