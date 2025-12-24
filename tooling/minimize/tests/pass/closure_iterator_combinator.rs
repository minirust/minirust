fn main() {
    let arr = [1, 2, 3, 4];
    assert!(arr.iter().all(|x| *x < 10));
    assert!(arr.iter().any(|x| *x < 10));
    assert!(arr.iter().map(|x| *x+1).fold(0, |x, y| x + y) == 14);
    assert!(*arr.iter().find(|x| **x > 3).unwrap() == 4);
    assert!(*arr.iter().skip_while(|x| **x < 3).next().unwrap() == 3);
}
