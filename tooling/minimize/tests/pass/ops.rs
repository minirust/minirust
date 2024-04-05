extern crate intrinsics;
use intrinsics::*;

fn black_box<T>(t: T) -> T { t }

fn main() {
    print(-black_box(-42));
    print(black_box(12) + 30);
    print(black_box(55) - 13);
    print(black_box(7) * 6);
    print(black_box(504) / 12);
    print(black_box(112) % 70);
    print(black_box(171) & 62);

    print(black_box(10) > 2);
    print(black_box(10) >= 2);
    print(black_box(10) < 2);
    print(black_box(10) <= 2);
    print(black_box(10) == 2);
    print(black_box(10) != 2);

    print(black_box(true) & true);
}
