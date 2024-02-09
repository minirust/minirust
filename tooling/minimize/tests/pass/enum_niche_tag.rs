
extern crate intrinsics;
use intrinsics::*;

fn print_opt_bool(b: Option<bool>) {
    match b {
        None => print(-1),
        Some(false) => print(0),
        Some(true) => print(1),
    }
}

enum Outer {
    V1(u32, Inner, u16),
    V2,
    V3,
}

#[repr(i16)]
enum Inner {
    V1 = -32767,
    V2 = -32768,
}

fn print_outer(o: Outer) {
    match o {
        Outer::V1(x, Inner::V1, y) => {
            print(0);
            print(x);
            print(y);
        },
        Outer::V1(x, Inner::V2, y) => {
            print(1);
            print(x);
            print(y);
        },
        Outer::V2 => print(2),
        Outer::V3 => print(3),
    }
}

fn main() {
    print_opt_bool(Some(true));
    print_opt_bool(Some(false));
    print_opt_bool(None);

    print_outer(Outer::V1(12, Inner::V1, 42));
    print_outer(Outer::V1(8888, Inner::V2, 127));
    print_outer(Outer::V2);
    print_outer(Outer::V3);
}
