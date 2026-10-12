#![feature(pattern_types)]
#![feature(pattern_type_macro)]
#![allow(internal_features)]

use core::mem::transmute;
use core::pattern_type;

type SmallU32 = pattern_type!(u32 is 1..=10);

fn main() {
    let invalid = 0u32;
    let _ub: SmallU32 = unsafe { transmute(invalid) };
}
