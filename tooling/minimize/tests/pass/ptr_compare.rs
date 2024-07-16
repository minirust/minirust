fn main() {
    let x = 2;
    let y = 2;
    let xptr = &x as *const i32;
    let yptr = &y as *const i32;
    assert!(xptr == xptr);
    assert!(xptr != yptr);
    assert!(xptr < yptr || xptr > yptr);
}
