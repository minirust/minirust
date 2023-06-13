extern crate intrinsics;
use intrinsics::*;

struct A<T> {
    x: T,
}

impl<T: std::fmt::Display + Copy> A<T> {
    fn foo(&self) {
        print(self.x);
    }
}

fn main() {
    let mut a: A<i32> = A { x: 20 };
    a.x += 1;
    a.foo();
}
