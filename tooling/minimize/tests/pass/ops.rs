use std::cmp::Ordering;

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
    assert!(unsafe { black_box(12_i32).unchecked_add(30) } == 42);
    assert!(unsafe { black_box(55_i32).unchecked_sub(13) } == 42);
    assert!(unsafe { black_box(7_i32).unchecked_mul(6) } == 42);
    // For unchecked shifts Rust only allows for u32 on right side 
    assert!(unsafe { black_box(i32::MAX).unchecked_shl(1u32) } == -2);
    assert!(unsafe { black_box(i32::MIN).unchecked_shl(1u32) } == 0);
    assert!(unsafe { black_box(-1_i32).unchecked_shr(1u32) } == -1);
    assert!(unsafe { black_box(84_i32).unchecked_shr(1u32) } == 42);
    assert!(black_box(42).cmp(&41) == Ordering::Greater);
    assert!(black_box(42).cmp(&42) == Ordering::Equal);
    assert!(black_box(42).cmp(&43) == Ordering::Less);

    assert!(black_box(41_i32).checked_add(1) == Some(42));
    assert!(black_box(i32::MAX).checked_add(1) == None);
    assert!(black_box(43_i32).checked_sub(1) == Some(42));
    assert!(black_box(i32::MIN).checked_sub(1) == None);
    assert!(black_box(21_i32).checked_mul(2) == Some(42));
    assert!(black_box(i32::MIN).checked_mul(-1) == None);

    assert!(black_box(41_i32).overflowing_add(1) == (42, false));
    assert!(black_box(i32::MAX).overflowing_add(1) == (i32::MIN, true));
    assert!(black_box(43_i32).overflowing_sub(1) == (42, false));
    assert!(black_box(i32::MIN).overflowing_sub(1) == (i32::MAX, true));
    assert!(black_box(21_i32).overflowing_mul(2) == (42, false));
    assert!(black_box(i32::MIN).overflowing_mul(-1) == (i32::MIN, true));

    assert!(black_box(42).cmp(&41) == Ordering::Greater);
    assert!(black_box(42).cmp(&42) == Ordering::Equal);
    assert!(black_box(42).cmp(&43) == Ordering::Less);

    assert!(black_box(10) > 2);
    assert!(black_box(10) >= 2);
    assert!(!(black_box(10) < 2));
    assert!(!(black_box(10) <= 2));
    assert!(!(black_box(10) == 2));
    assert!(black_box(10) != 2);

    assert!(black_box(true) & true);
    assert!(black_box(false) | true);
    assert!(black_box(false) ^ true);
    assert!(black_box(false) < true);
    assert!(black_box(false) <= true);
    assert!(black_box(false) == false);
    assert!(black_box(true) >= false);
    assert!(black_box(true) > false);
}
