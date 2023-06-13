extern crate intrinsics;
use intrinsics::*;

// previously this failed as a `(2, ())` tuple is stored as a ConstValue::Scalar, which wasn't expected.

const X: (u8, ()) = (2, ());
static Y: (u8, ()) = (2, ());

fn main() {
    let x = X;
    let y = Y;
    let z = (2, ());

    print(x.0);
    print(y.0);
    print(z.0);
}

