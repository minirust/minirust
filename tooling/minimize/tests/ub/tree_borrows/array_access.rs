extern crate intrinsics;
use intrinsics::*;

//@ compile-flags: --minimize-tree-borrows
fn main() {
    unsafe {
        let mut data = [0u8, 1, 2, 3];
        let x = &mut data[0] as *mut u8; // (x, [R, R, R, R])
        let y = &mut *x; // (x, [R, R, R, R]) -> (y, [R, R, R, R])
        let yraw = y as *mut u8;

        *x.add(1) = 42; // (x, [R, A, R, R]) -> (y, [R, D, R, R])

        print(*yraw.add(0));
        print(*yraw.add(2));
        print(*yraw.add(3));

        *yraw.add(1) = 57;
    }
}
