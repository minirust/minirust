use std::mem;

fn main() {
    assert!(4 == mem::align_of_val(&0i32));
    assert!(4 == mem::align_of_val(&[0; 0]));
    assert!(4 == mem::align_of_val(&[0; 4]));
}
