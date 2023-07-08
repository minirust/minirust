extern crate intrinsics;
use intrinsics::*;

fn main() {
    let mut x: usize = 1;

    let ptr = (&mut x) as *mut usize;

    let v = unsafe { atomic_read(ptr) };
    print(v);

    unsafe { atomic_write(ptr, 2) };
    print(x);

    let v = unsafe { compare_exchange(ptr, 2, 3) };
    print(x);
    print(v);

    let v = unsafe { compare_exchange(ptr, 2, 4) };
    print(x);
    print(v);
}
