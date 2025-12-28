fn add(a: i32, b:i32) -> i32 {
    a + b
}

fn main() {
    let f: fn(i32, i32) -> i32 = add; 
    assert!(f(1, 2) == 3);
}
