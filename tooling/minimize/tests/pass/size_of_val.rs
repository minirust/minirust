use std::mem;
use std::ptr;

fn main() {
    assert!(4 == mem::size_of_val(&5i32));
    assert!(0 == mem::size_of_val(&()));

    assert!(8 == mem::size_of_val(&[0_u32; 2]));
    assert!(12 == mem::size_of_val(&[0_u32; 3]));
    assert!(0 == mem::size_of_val(&[0_u32; 0]));

    assert!(12 == mem::size_of_val(&[0_u32; 3] as &[u32]));
    assert!(0 == mem::size_of_val(&[0_u32; 0] as &[u32]));
    assert!(11 == mem::size_of_val("Hello World"));

    assert!(mem::size_of_val(&ptr::null::<i32>()) == mem::size_of_val(&ptr::null_mut::<i32>()));

    #[repr(C)]
    struct FieldStruct {
        first: u8,
        second: u16,
        third: u8,
    }

    assert!(6 == mem::size_of_val(&FieldStruct { first: 0, second: 0, third: 0 }));

    #[repr(C)]
    struct TupleStruct(u8, u16, u8);
    assert!(6 == mem::size_of_val(&TupleStruct(0, 0, 0)));

    #[repr(C)]
    struct FieldStructOptimized {
        first: u8,
        third: u8,
        second: u16,
    }
    assert!(4 == mem::size_of_val(&FieldStructOptimized { first: 0, second: 0, third: 0 }));

    #[repr(C)]
    union ExampleUnion {
        smaller: u8,
        larger: u16,
    }
    assert!(2 == mem::size_of_val(&ExampleUnion { smaller: 0 }));
    assert!(2 == mem::size_of_val(&ExampleUnion { larger: 0 }));
}
