fn main() {
    let p = {
        let b = 42;
        &b as *const i32 as *const (u8, u8, u8, u8)
    };
    unsafe {
        // Without the projection, this is fine.
        let _ = *p;
    }
}
