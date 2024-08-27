//! This test case ensures that enums correctly mark their used bytes for unions and don't mark padding.
//! It creates a union with all initialized bytes, and then after a copy tries to read all bytes.
//! The read should fail as padding will get uninitialized during the copy.
//! For alignment reasons the bytes are read as `u16`, as the enum is 2-byte aligned.

use std::mem::transmute;

#[derive(Clone, Copy)]
#[repr(u16)]
#[allow(unused)]
enum Inner {
    WithData(u16, u8),
    Empty,
}

#[derive(Clone, Copy)]
union TestUnion {
    _data: Inner
}

// Shorthand for an array of `u8` with the same size of the union.
type UnionAsArray = [u8; std::mem::size_of::<TestUnion>()];

fn get_union_as_array(u: TestUnion) -> UnionAsArray {
    unsafe { transmute::<TestUnion, UnionAsArray>(u) }
}


fn main() {
    let zero = [0u8; std::mem::size_of::<TestUnion>()];
    let u: TestUnion = unsafe { transmute::<UnionAsArray, TestUnion>(zero) };
    let _shorts = get_union_as_array(u);
}
