fn main() {
    let x = 2;
    let y = &x as *const i32;
    let z = unsafe { *y };
    assert!(z == 2);
}
