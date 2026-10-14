#![feature(pattern_types)]
#![feature(pattern_type_macro)]
#![feature(pattern_type_range_trait)]
#![feature(const_trait_impl)]
#![feature(core_intrinsics)]
#![allow(internal_features, unnecessary_transmutes)]

extern crate intrinsics;
use intrinsics::*;

use core::mem::{align_of_val, size_of, size_of_val, transmute};
use core::num::{NonZeroI32, NonZeroU32};
use core::pattern_type;
use core::ptr::NonNull;

type SmallU32 = pattern_type!(u32 is 1..=10);
type SignedOrPat = pattern_type!(i32 is ..=-5 | 10..);
type NonNullPtr<T> = pattern_type!(*const T is !null);
type NestedNonNullPtr<T> = pattern_type!(NonNullPtr<T> is !null);
type NonNullSlicePtr<T> = pattern_type!(*const [T] is !null);

fn test_range_pat() {
    assert!(size_of::<Option<SmallU32>>() == size_of::<u32>());
    let s: SmallU32 = unsafe { transmute(7u32) };
    let back: u32 = unsafe { transmute(s) };
    print(back);

    let opt_some: Option<SmallU32> = Some(s);
    let opt_none: Option<SmallU32> = unsafe { transmute(0u32) };
    assert!(opt_some.is_some());
    assert!(opt_none.is_none());
}

fn test_or_pat() {
    let neg: SignedOrPat = unsafe { transmute(-42i32) };
    let pos: SignedOrPat = unsafe { transmute(99i32) };
    let neg_val: i32 = unsafe { transmute(neg) };
    let pos_val: i32 = unsafe { transmute(pos) };
    print(neg_val);
    print(pos_val);
}

fn test_nonnull_and_stdlib() {
    let x: i32 = 123;
    let raw: *const i32 = &raw const x;
    let nonnull_pat: NestedNonNullPtr<i32> = unsafe { transmute(raw) };
    let raw_back: *const i32 = unsafe { transmute(nonnull_pat) };
    print(unsafe { *raw_back });

    let nn = NonNull::from(&x);
    print(unsafe { *nn.as_ptr() });

    let nzu = NonZeroU32::new(55).unwrap();
    let nzi = NonZeroI32::new(-55).unwrap();
    print(nzu.get());
    print(nzi.get());

    let c: char = unsafe { transmute(65u32) };
    let c_back: u32 = unsafe { transmute(c) };
    print(c_back);
}

#[repr(C)]
union PatUnion {
    s: SmallU32,
    u: u32,
}

fn pass_and_return(s: SmallU32, p: NonNullPtr<i32>) -> (SmallU32, NonNullPtr<i32>) {
    (s, p)
}

fn test_places_aggregates_and_calls() {
    let s: SmallU32 = unsafe { transmute(5u32) };
    let x: i32 = 42;
    let p: NonNullPtr<i32> = unsafe { transmute(&raw const x) };

    // Moves, copies, references, raw pointers
    let s2 = s;
    let r = &s2;
    let s3 = *r;
    let rp = &raw const s3;
    let s4 = unsafe { *rp };

    // Tuples, arrays, array repeats, unions, and fn calls
    let (s5, p2) = pass_and_return(s4, p);
    let arr = [s5; 3];
    let u = PatUnion { s: arr[1] };
    assert!(unsafe { u.u } == 5);
    let p_back: *const i32 = unsafe { transmute(p2) };
    assert!(unsafe { *p_back } == 42);

    // Wide `!null` slice pointer and niche-optimized Option
    let slice_arr = [10i32, 20, 30];
    let ps: NonNullSlicePtr<i32> = unsafe { transmute(&slice_arr[..] as *const [i32]) };
    let opt_ps: Option<NonNullSlicePtr<i32>> = Some(ps);
    assert!(size_of::<Option<NonNullSlicePtr<i32>>>() == size_of::<*const [i32]>());
    assert!(opt_ps.is_some());

    // Supported intrinsics
    assert!(unsafe { core::intrinsics::raw_eq(&s, &s5) });
    assert!(unsafe { core::intrinsics::raw_eq(&p, &p2) });
    assert!(size_of_val(&s) == 4);
    assert!(align_of_val(&s) == 4);
    assert!(size_of_val(&ps) == 16);
    assert!(core::intrinsics::discriminant_value(&opt_ps) == 1);
    core::intrinsics::assert_inhabited::<SmallU32>();
    core::intrinsics::assert_inhabited::<NonNullPtr<i32>>();
}

#[inline(never)]
fn id_small(s: SmallU32) -> SmallU32 {
    s
}

#[inline(never)]
fn id_char(c: char) -> char {
    c
}

fn test_const_pat() {
    const C_SMALL: SmallU32 = unsafe { transmute(5u32) };
    let s = id_small(C_SMALL);
    let u: u32 = unsafe { transmute(s) };
    assert!(u == 5);

    let c = id_char('A');
    assert!(c as u32 == 65);
}

fn test_char_cmp() {
    let c1: char = id_char(unsafe { transmute(65u32) });
    let c2: char = id_char(unsafe { transmute(66u32) });
    assert!(c1 == c1);
    assert!(c1 != c2);
    assert!(c1 < c2);
}

fn test_char_cast() {
    let c: char = id_char(unsafe { transmute(65u32) });
    let u: u32 = c as u32;
    assert!(u == 65);
    let b: u8 = u as u8;
    let c2: char = id_char(b as char);
    let u2: u32 = unsafe { transmute(c2) };
    assert!(u2 == 65);
}

fn test_char_match() {
    let c: char = id_char(unsafe { transmute(65u32) });
    let matched = match c {
        'A' => true,
        _ => false,
    };
    assert!(matched);
}

fn main() {
    test_range_pat();
    test_or_pat();
    test_nonnull_and_stdlib();
    test_places_aggregates_and_calls();

    test_const_pat();
    test_char_cmp();
    test_char_cast();
    test_char_match();
}
