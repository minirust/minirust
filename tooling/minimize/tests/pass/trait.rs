trait Trait {
    fn function(&self);
}

fn generic<T: Trait>(x: &T) {
    x.function();
}

impl Trait for i32 {
    fn function(&self) {}
}

fn main() {
    generic(&42i32);
}
