fn is_even(x: i32) -> bool {
    x % 2 == 0
}

fn main() {
    let o = Some(42);
    assert!(o.is_some_and(is_even));
}