fn main() {
    let x = -1;
    let y = &x as *const i32 as *const u32;
    assert!(unsafe { *y } == u32::MAX);
}
