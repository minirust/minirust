fn black_box<T>(t: T) -> T { t }

fn main() {
    assert!(-black_box(-42) == 42);
    assert!(!black_box(-43) == 42);
    assert!(!black_box(1_u32) == (u32::MAX - 1));
    assert!(!black_box(-1) == 0);
    assert!(!black_box(i32::MIN) == i32::MAX);
    assert!(black_box(12) + 30 == 42);
    assert!(black_box(55) - 13 == 42);
    assert!(black_box(7) * 6 == 42);
    assert!(black_box(504) / 12 == 42);
    assert!(black_box(112) % 70 == 42);
    assert!(black_box(i32::MAX) << 1 == -2);
    assert!(black_box(i32::MIN) << 1u8 == 0);
    assert!(black_box(-1) >> 1 == -1);
    assert!(black_box(84) >> 1u8 == 42);
    assert!(black_box(171) & 62 == 42);
    assert!(black_box(10) | 34 == 42);
    assert!(black_box(36) ^ 14 == 42);

    assert!(black_box(10) > 2);
    assert!(black_box(10) >= 2);
    assert!(!(black_box(10) < 2));
    assert!(!(black_box(10) <= 2));
    assert!(!(black_box(10) == 2));
    assert!(black_box(10) != 2);

    assert!(black_box(true) & true);
    assert!(black_box(false) | true);
    assert!(black_box(false) ^ true)
}
