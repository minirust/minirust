extern crate intrinsics;
use intrinsics::*;
include!("../helper/eq.rs");

fn main() {
    print(is_zero_u8(0));
    print(is_zero_u8(1));
    print(is_zero_u8(14));
}
