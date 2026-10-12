#![feature(pattern_types)]
#![feature(pattern_type_macro)]
#![allow(internal_features)]

use core::mem::transmute;
use core::pattern_type;
use core::ptr;

type NonNullI32 = pattern_type!(*const i32 is !null);

fn main() {
    let p = ptr::null::<i32>();
    let _ub: NonNullI32 = unsafe { transmute(p) };
}
