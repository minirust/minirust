#![feature(arbitrary_self_types_pointers)]

// Dispatch on Box cannot be tested due to various functions without optimized_mir, see #175

trait A {
    fn by_ref(&self) -> usize;
    fn by_mut_ref(&mut self) -> usize;
    fn by_raw(self: *const Self) -> bool;
    // fn by_box(self: Box<Self>) -> usize;
}

impl A for usize {
    fn by_ref(&self) -> usize {
        *self
    }
    fn by_mut_ref(&mut self) -> usize {
        *self
    }
    fn by_raw(self: *const Self) -> bool {
        true
    }
    // fn by_box(self: Box<Self>) -> usize {
    //     *self
    // }
}

impl A for u8 {
    fn by_ref(&self) -> usize {
        *self as usize
    }
    fn by_mut_ref(&mut self) -> usize {
        *self as usize
    }
    fn by_raw(self: *const Self) -> bool {
        false
    }
    // fn by_box(self: Box<Self>) -> usize {
    //     *self as usize
    // }
}

fn main() {
    let mut x: usize = 42;
    let y1: &mut dyn A = &mut x;
    let y2: &mut dyn A = &mut 8_u8;
    assert!(core::mem::size_of_val(y1) == 8);
    assert!(core::mem::align_of_val(y1) == 8);
    assert!(core::mem::size_of_val(y2) == 1);
    assert!(core::mem::align_of_val(y2) == 1);

    assert!(y1.by_ref() == 42);
    assert!(y1.by_mut_ref() == 42);
    assert!(y2.by_ref() == 8);
    assert!(y2.by_mut_ref() == 8);

    assert!((core::ptr::null::<usize>() as *const dyn A).by_raw());
    assert!(!(core::ptr::null::<u8>() as *const dyn A).by_raw());

    // let b: Box<dyn A> = Box::new(1337_usize);
    // assert!(b.by_ref() == 1337);
    // assert!(b.by_box() == 1337);
}
