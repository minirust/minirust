fn main() {
    unsafe {
        let x = 12u8;
        let x_ptr: *const u8 = &x;
        let cast_ptr = x_ptr as *const Option<bool>;
        // Miri does *not* make this UB because it doesn't realize the discriminant
        // is in outside the range of possible niche values.
        let _val = matches!(*cast_ptr, None);
    }
}
