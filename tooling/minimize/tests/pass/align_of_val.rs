use std::mem;

fn main() {
    assert!(4 == mem::align_of_val(&0i32));
    assert!(4 == mem::align_of_val(&[0_u32; 0]));
    assert!(2 == mem::align_of_val(&[0_u16; 4]));

    assert!(4 == mem::align_of_val(&[0_u32; 0] as &[u32]));
    assert!(2 == mem::align_of_val(&[0_u16; 4] as &[u16]));
}
