extern crate intrinsics;
use intrinsics::*;

const X0: [i32; 0] = [];
static Y0: [i32; 0] = [];

const X1: [i32; 1] = [2];
static Y1: [i32; 1] = [2];

fn main() {
    let _x = X0;
    let _y = Y0;
    let _z: [i32; 0] = [];

    let x = X1;
    let y = Y1;
    let z: [i32; 1] = [2];

    print(x[0]);
    print(y[0]);
    print(z[0]);
}
