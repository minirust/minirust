//@ compile-flags: -C panic=abort

extern crate intrinsics;
use intrinsics::*;

struct A {
    x: i32,
}

impl Drop for A {
    fn drop(&mut self) {
        print(self.x);
    }
}

#[allow(unconditional_recursion)]
fn f(elem: A) {
    print(100 / elem.x); // Causes a panic if `elem.x == 0`
    let next = A { x: elem.x - 1 };
    f(next);
    print(-1); // Unreachable
}

fn main() {
    let a = A { x: 5 };
    f(a);
    print(-1); // Unreachable
}
