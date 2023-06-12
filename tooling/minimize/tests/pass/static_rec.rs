extern crate intrinsics;
use intrinsics::*;

struct S<'a> {
    this: &'a S<'a>,
    val: i32,
}

static RECURSIVE: S = S {
    this: &RECURSIVE,
    val: 42,
};

fn main() {
    print(RECURSIVE.val);
    print(RECURSIVE.this.val);
}
