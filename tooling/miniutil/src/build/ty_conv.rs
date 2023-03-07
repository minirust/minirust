//! This allows you to convert Rust types to minirust types conveniently.

use crate::build::*;

pub trait TypeConv {
    fn get_type() -> Type;
    fn get_align() -> Align;
    fn get_size() -> Size;

    fn get_ptype() -> PlaceType {
        PlaceType {
            ty: Self::get_type(),
            align: Self::get_align(),
        }
    }

    fn get_layout() -> Layout {
        Layout {
            size: Self::get_size(),
            align: Self::get_align(),
            inhabited: true, // currently there are no uninhabited types in minirust; Type::Enum is not yet supported!
        }
    }
}

macro_rules! type_conv_impl {
    ($ty:ty, $signed:expr, $size:expr, $align:expr) => {
        impl TypeConv for $ty {
            fn get_type() -> Type {
                Type::Int(IntType { signed: $signed, size: size($size)})
            }
            fn get_align() -> Align {
                align($align)
            }
            fn get_size() -> Size {
                size($size)
            }
        }
    }
}

type_conv_impl!(u8, Unsigned, 1, 1);
type_conv_impl!(u16, Unsigned, 2, 2);
type_conv_impl!(u32, Unsigned, 4, 4);
type_conv_impl!(u64, Unsigned, 8, 8);
type_conv_impl!(u128, Unsigned, 16, 8);

type_conv_impl!(i8, Signed, 1, 1);
type_conv_impl!(i16, Signed, 2, 2);
type_conv_impl!(i32, Signed, 4, 4);
type_conv_impl!(i64, Signed, 8, 8);
type_conv_impl!(i128, Signed, 16, 8);

type_conv_impl!(usize, Unsigned, 8, 8);
type_conv_impl!(isize, Signed, 8, 8);

impl<T: TypeConv> TypeConv for *const T {
    fn get_type() -> Type { raw_ptr_ty(T::get_layout()) }
    fn get_align() -> Align { align(8) }
    fn get_size() -> Size { size(8) }
}

impl<T: TypeConv> TypeConv for *mut T {
    fn get_type() -> Type { raw_ptr_ty(T::get_layout()) }
    fn get_align() -> Align { align(8) }
    fn get_size() -> Size { size(8) }
}

impl<T: TypeConv> TypeConv for &T {
    fn get_type() -> Type { ref_ty(T::get_layout()) }
    fn get_align() -> Align { align(8) }
    fn get_size() -> Size { size(8) }
}

impl<T: TypeConv> TypeConv for &mut T {
    fn get_type() -> Type { ref_mut_ty(T::get_layout()) }
    fn get_align() -> Align { align(8) }
    fn get_size() -> Size { size(8) }
}

impl TypeConv for bool {
    fn get_type() -> Type { bool_ty() }
    fn get_align() -> Align { align(1) }
    fn get_size() -> Size { size(1) }
}

impl<T: TypeConv, const N: usize> TypeConv for [T; N] {
    fn get_type() -> Type { array_ty(T::get_type(), N) }

    fn get_align() -> Align { T::get_align() }
    fn get_size() -> Size { T::get_size() * N.into() }
}

impl TypeConv for () {
    fn get_type() -> Type { tuple_ty(&[], size(0)) }
    fn get_align() -> Align { align(1) }
    fn get_size() -> Size { size(0) }
}
