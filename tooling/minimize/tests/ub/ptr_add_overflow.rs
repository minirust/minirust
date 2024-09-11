fn main() {
    let data = [1u16, 2, 3, 4, 5];
    let first = &data[0] as *const u16;
    unsafe {
        let _ptr = first.add(2).add(-1i32 as usize);
    }
}
