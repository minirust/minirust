//@revisions: ub_check no_ub_check
//@[no_ub_check]compile-flags: -Zub-checks=no


#![feature(core_intrinsics)]
#![allow(internal_features)]
use std::mem;

fn size_of() {
    // Some primitives
    assert!(4 == mem::size_of::<i32>());
    assert!(8 == mem::size_of::<f64>());
    assert!(0 == mem::size_of::<()>());

    // Some arrays
    assert!(8 == mem::size_of::<[i32; 2]>());
    assert!(12 == mem::size_of::<[i32; 3]>());
    assert!(0 == mem::size_of::<[i32; 0]>());


    // Pointer size equality
    assert!(mem::size_of::<&i32>() == mem::size_of::<*const i32>());
    assert!(mem::size_of::<&i32>() == mem::size_of::<Box<i32>>());

    #[repr(C)]
    struct FieldStruct {
        first: u8,
        second: u16,
        third: u8
    }
    assert!(6 == mem::size_of::<FieldStruct>());

    #[repr(C)]
    struct TupleStruct(u8, u16, u8);
    assert!(6 == mem::size_of::<TupleStruct>());

    #[repr(C)]
    struct FieldStructOptimized {
        first: u8,
        third: u8,
        second: u16
    }
    assert!(4 == mem::size_of::<FieldStructOptimized>());

    #[repr(C)]
    union ExampleUnion {
        smaller: u8,
        larger: u16
    }
    assert!(2 == mem::size_of::<ExampleUnion>());
}

fn align_of() {
    assert!(4 == mem::align_of::<i32>());
    assert!(4 == mem::align_of::<[i32; 0]>());
    assert!(4 == mem::align_of::<[i32; 4]>());
}


fn offset_of() {
    #[repr(C)]
    struct FieldStruct {
        first: u8,
        second: u16,
        third: u8
    }

    assert!(0 == mem::offset_of!(FieldStruct, first));
    assert!(2 == mem::offset_of!(FieldStruct, second));
    assert!(4 == mem::offset_of!(FieldStruct, third));

    #[repr(C)]
    struct NestedA {
        b: NestedB
    }

    #[repr(C)]
    struct NestedB(u8);
    assert!(0 == mem::offset_of!(NestedA, b.0));
}

fn ub_check() {
    assert!(cfg!(ub_check) == core::intrinsics::ub_checks());
}

fn main() {
    size_of();
    align_of();
    offset_of();
    ub_check();
}
