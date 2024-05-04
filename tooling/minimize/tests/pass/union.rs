extern crate intrinsics;
use intrinsics::*;

union A {
    f1: u32,
    f2: (),
}

#[allow(unused)]
union B {
    f1: (u8, u16),
    f2: u8,
}


#[derive(Clone, Copy)]
union EnumUnion {
    data: Option<i32>,
}

fn extract_some_int(u: EnumUnion) -> i32 {
    unsafe {
        match u.data {
            Some(i) => i,
            None => unreachable!(),
        }
    }
}


#[derive(Clone, Copy, PartialEq, Eq)]
struct TestStruct(i8, i16);

#[derive(Clone, Copy)]
union StructUnion {
    data: TestStruct,
}

fn extract_struct(u: StructUnion) -> TestStruct {
    unsafe { u.data }
}


#[derive(Clone, Copy)]
union ArrayUnion {
    data: [[i8; 3]; 2]
}

fn extract_array(u: ArrayUnion) -> [[i8; 3]; 2] {
    unsafe { u.data }
}



fn main() {
    let mut x = A { f2: ()};
    x.f1 = 20;
    unsafe {
        print(x.f1);
    }

    let _y = B { f2: 0 };

    // Make sure the value in `data` is preserved for enums, structs and arrays.
    let u = EnumUnion { data: Some(42) };
    assert!(extract_some_int(u) == 42);

    let u = StructUnion { data: TestStruct(12, 1200) };
    assert!(extract_struct(u) == TestStruct(12, 1200));
    
    let u = ArrayUnion { data: [[42;3], [12;3]] };
    // FIXME: this still fails to translate (needs Rvalue::Cast(Transmute))
    // assert!(extract_array(u) == [[42;3], [12;3]]);
    let a = extract_array(u);
    assert!(a[0][1] == 42);
    assert!(a[0][2] == 42);
    assert!(a[1][0] == 12);
    assert!(a[1][1] == 12);
    assert!(a[1][2] == 12);
}
