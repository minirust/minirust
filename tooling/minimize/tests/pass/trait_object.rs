trait A {
    fn foo(&self) -> usize;
}

impl A for usize {
    fn foo(&self) -> usize {
        *self
    }
}

impl A for u8 {
    fn foo(&self) -> usize {
        *self as usize
    }
}

fn main() {
    let x: usize = 42;
    let y1: &dyn A = &x;
    let y2: &dyn A = &8_u8;
    assert!(core::mem::size_of_val(y1) == 8);
    assert!(core::mem::align_of_val(y1) == 8);
    assert!(core::mem::size_of_val(y2) == 1);
    assert!(core::mem::align_of_val(y2) == 1);
    assert!(y1.foo() == x);
    assert!(y2.foo() == 8);
}
