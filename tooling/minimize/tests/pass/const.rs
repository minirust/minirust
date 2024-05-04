enum Outer {
    Left(Inner),
    Right(i128),
}

enum Inner {
    Val(i32),
}

enum NegDiscr {
    ONE = -1,
    TWO = -2,
}

struct S {
    x: i32,
    y: Inner,
}

const X: (i64, i64) = (2, 3);
const INT: i32 = 41;
const FALSE: bool = false;
const NEG_ONE: NegDiscr = NegDiscr::ONE;
const NEG_TWO: NegDiscr = NegDiscr::TWO;
const ENUM: Outer = Outer::Left(Inner::Val(42));
const RAW: *const i32 = 4_i32 as *const i32;
const PTR: &(&(Outer, Outer), i32) = &(&(Outer::Left(Inner::Val(20)), Outer::Right(20)), 2);
const TUPLE: (i32, Inner) = (40, Inner::Val(2));
const ARRAY: [Inner; 3] = [Inner::Val(20), Inner::Val(20), Inner::Val(2)];
const STRUCT: S = S { x: 40, y: Inner::Val(2) };

fn main() {
    let x = X;
    assert!(x.0 == 2);
    assert!(x.1 == 3);

    assert!(INT + 1 == 42);
    assert!(!FALSE);

    assert!(RAW as usize == 4);

    // check set negative discriminants
    assert!(NEG_ONE as i32 == -1);
    assert!(NEG_TWO as i32 == -2);

    // check Pointer with provenance and recursive evaluation
    let &(&(Outer::Left(Inner::Val(x)), Outer::Right(y)), z) = PTR else { unreachable!() };
    assert!(x == 20);
    assert!(y == 20);
    assert!(z == 2);

    let (x, Inner::Val(y)) = TUPLE;
    assert!(x == 40);
    assert!(y == 2);

    let Outer::Left(Inner::Val(x)) = ENUM else { unreachable!() };
    assert!(x == 42);

    let Inner::Val(x) = ARRAY[0];
    assert!(x == 20);
    let Inner::Val(x) = ARRAY[1];
    assert!(x == 20);
    let Inner::Val(x) = ARRAY[2];
    assert!(x == 2);

    let S { x, y: Inner::Val(y) } = STRUCT;
    assert!(x == 40);
    assert!(y == 2);

    // This involves some interesting constants as well.
    assert!(!(() > ()));
}
