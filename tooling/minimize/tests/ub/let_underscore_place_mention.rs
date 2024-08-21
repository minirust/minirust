fn main() {
    let p = {
        let b = 42;
        &b as *const i32 as *const (u8, u8, u8, u8)
    };
    unsafe {
        // Due to the projection, this is not fine.
        // (FIXME: the error is pretty bad, it doesn't mention "offset"/projection...)
        let _ = (*p).1;
    }
}
