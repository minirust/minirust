trait A {
    #[allow(dead_code)]
    fn foo(&self) -> usize;
}

trait B {}

impl A for usize {
    fn foo(&self) -> usize {
        *self
    }
}

impl B for usize {}

fn main() {
    let a: &dyn A = &42_usize;
    let _b = unsafe { core::mem::transmute::<&dyn A, &dyn B>(a) };
}
