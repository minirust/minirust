//@compile-flags: -Coverflow-checks=no
extern crate intrinsics;
use intrinsics::*;

fn black_box<T>(t: T) -> T { t }

fn main() {
    // some other overflowing tests happen in casts.rs
    print(black_box(128u8) + 128u8); // 0
    print(black_box(128u8) * 2); // 0
    print(black_box(0u8) - 1u8); // 255

    print(black_box(i32::MAX) + 1); // -2147483648
    print(i32::MIN); // same as above

    print(black_box(i32::MIN) - 1); // 2147483647
    print(i32::MAX); // same as above
}
