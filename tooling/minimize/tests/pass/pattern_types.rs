#![feature(pattern_types)]
#![feature(pattern_type_macro)]
#![feature(pattern_type_range_trait)]
#![feature(const_trait_impl)]
#![allow(internal_features)]

extern crate intrinsics;
use intrinsics::*;

use core::mem::{size_of, transmute};
use core::num::{NonZeroI32, NonZeroU32};
use core::pattern_type;
use core::ptr::NonNull;

type SmallU32 = pattern_type!(u32 is 1..=10);
type SignedOrPat = pattern_type!(i32 is ..=-5 | 10..);
type NonNullPtr<T> = pattern_type!(*const T is !null);
type NestedNonNullPtr<T> = pattern_type!(NonNullPtr<T> is !null);

fn main() {
    // 1. Range pattern type (`1..=10`) & niche layout optimization
    assert!(size_of::<Option<SmallU32>>() == size_of::<u32>());
    let s: SmallU32 = unsafe { transmute(7u32) };
    let back: u32 = unsafe { transmute(s) };
    print(back);

    let opt_some: Option<SmallU32> = Some(s);
    let opt_none: Option<SmallU32> = unsafe { transmute(0u32) };
    assert!(opt_some.is_some());
    assert!(opt_none.is_none());

    // 2. Signed OR-range pattern type (`..=-5 | 10..`)
    let neg: SignedOrPat = unsafe { transmute(-42i32) };
    let pos: SignedOrPat = unsafe { transmute(99i32) };
    let neg_val: i32 = unsafe { transmute(neg) };
    let pos_val: i32 = unsafe { transmute(pos) };
    print(neg_val);
    print(pos_val);

    // 3. `!null` raw pointer pattern type, nested `!null`, and `NonNull<T>`
    let x: i32 = 123;
    let raw: *const i32 = &raw const x;
    let nonnull_pat: NestedNonNullPtr<i32> = unsafe { transmute(raw) };
    let raw_back: *const i32 = unsafe { transmute(nonnull_pat) };
    print(unsafe { *raw_back });

    let nn = NonNull::from(&x);
    print(unsafe { *nn.as_ptr() });

    // 4. `NonZeroU32` / `NonZeroI32` and `char`
    let nzu = NonZeroU32::new(55).unwrap();
    let nzi = NonZeroI32::new(-55).unwrap();
    print(nzu.get());
    print(nzi.get());

    let c: char = 'A';
    print(c as u32);
}
