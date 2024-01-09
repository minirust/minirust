#![feature(never_type)]

extern crate intrinsics;
use intrinsics::*;

enum A {
    A1(u8),
    A2
}

fn print_a(a: &A) {
    if let A::A1(x) = a {
        print(*x)
    } else {
        print(-1)
    }
}

fn print_a_match(a: &A) {
    match a {
        A::A1(x) => print(*x),
        A::A2 => print(-1),
    }
}

#[repr(i16)]
enum I16Repr {
    Min = i16::MIN,
    Minus1 = -1,
    Zero = 0,
    Max = i16::MAX,
}

fn print_i16_repr(a: I16Repr) {
    match a {
        I16Repr::Min => print(-2),
        I16Repr::Minus1 => print(-1),
        I16Repr::Zero => print(0),
        I16Repr::Max => print(1),
    }
}

fn main() {
    let x = A::A1(12);
    print_a(&x);
    print_a_match(&x);
    let x = A::A2;
    print_a(&x);
    print_a_match(&x);

    print_i16_repr(I16Repr::Min);
    print_i16_repr(I16Repr::Minus1);
    print_i16_repr(I16Repr::Zero);
    print_i16_repr(I16Repr::Max);

    // While this is not going to run it is forcing the minimizer to minimize `!`.
    if false {
        unsafe {
            let x = 0u8;
            let x_ptr: *const u8 = &x;
            let _ = *(x_ptr as *const !);
        }
    };
}
