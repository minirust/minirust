extern crate intrinsics;
use intrinsics::*;

fn main() {
    let mut x: u32 = 1;

    let ptr = (&mut x) as *mut u32;

    let v = unsafe { atomic_load(ptr) };
    print(v);

    unsafe { atomic_store(ptr, 2) };
    print(x);

    let v = unsafe { compare_exchange(ptr, 2, 3) };
    print(x);
    print(v);

    let v = unsafe { compare_exchange(ptr, 2, 4) };
    print(x);
    print(v);

    let v = unsafe { atomic_add(ptr, 3) };
    print(x);
    print(v);
    
    let v = unsafe { atomic_sub(ptr, 4) };
    print(x);
    print(v);
}
