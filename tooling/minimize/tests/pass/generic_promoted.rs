extern crate intrinsics;
use intrinsics::*;

fn main() {
    print(*generic_promoted::<i8>());
    print(*generic_promoted::<i16>());
}

fn generic_promoted<T>() -> &'static usize {
    &std::mem::size_of::<T>()
}
