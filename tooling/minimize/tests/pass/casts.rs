extern crate intrinsics;
use intrinsics::*;

fn black_box<T>(t: T) -> T { t }

fn main() {
    print(black_box(0u32) as u8); // 0
    print(black_box(256u32 + 42) as u8); // 42
    print(black_box(256i16) as u8); // 0
    print(black_box(u8::MAX) as i8); // -1
    print(black_box(-1i8) as u8); // 255
    print(black_box(24u8) as i128); // 24
    print(black_box(true as u8)); // 1
    print(black_box(false as i64)); // 0

    let x = 2;
    let addr = &x as *const i32 as usize;
    print(addr - addr / 4 * 4); // 0, because addr%4 == 0 due to alignment.

    // TODO also test int to ptr casts when they are supported.
}
