trait A {
    fn foo(&self) -> usize;
}

impl A for usize {
    fn foo(&self) -> usize {
        *self
    }
}

fn main() {
    let x: usize = 42;
    let y: &dyn A = &x;
    //assert!(y.foo() == x);
}
