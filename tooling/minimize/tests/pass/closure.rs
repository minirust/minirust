fn main() {
    let y = 2;
    let z = 3;
    let f = |x| x + y + z;
    let g = || y + z;
    assert!(f(1) == 6);
    assert!(g() == 5);
}
