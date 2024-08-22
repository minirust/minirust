//! This allows you to convert Rust types to MiniRust types conveniently.

use crate::build::*;

/// Converts a Rust type to a MiniRust type.
/// Example usage: `let x: Type = <usize>::get_type();`
pub trait TypeConv {
    fn get_type() -> Type;

    // Convenience methods, these should not be overridden.
    fn get_size() -> Size {
        Self::get_type().size::<DefaultTarget>()
    }
    fn get_align() -> Align {
        Self::get_type().align::<DefaultTarget>()
    }

    const FREEZE: bool = true;
}

macro_rules! type_conv_int_impl {
    ($ty:ty, $signed:expr, $size:expr) => {
        impl TypeConv for $ty {
            fn get_type() -> Type {
                Type::Int(IntType { signed: $signed, size: $size })
            }
        }
    };
}

type_conv_int_impl!(u8, Unsigned, size(1));
type_conv_int_impl!(u16, Unsigned, size(2));
type_conv_int_impl!(u32, Unsigned, size(4));
type_conv_int_impl!(u64, Unsigned, size(8));
type_conv_int_impl!(u128, Unsigned, size(16));

type_conv_int_impl!(i8, Signed, size(1));
type_conv_int_impl!(i16, Signed, size(2));
type_conv_int_impl!(i32, Signed, size(4));
type_conv_int_impl!(i64, Signed, size(8));
type_conv_int_impl!(i128, Signed, size(16));

// We use `BasicMemory` to run a Program (see the `run` module),
// hence we have to use its PTR_SIZE for `usize` and `isize`.
type_conv_int_impl!(usize, Unsigned, DefaultTarget::PTR_SIZE);
type_conv_int_impl!(isize, Signed, DefaultTarget::PTR_SIZE);

impl<T: TypeConv + ?Sized> TypeConv for *const T {
    fn get_type() -> Type {
        raw_ptr_ty(T::get_type().meta_kind())
    }
}

impl<T: TypeConv + ?Sized> TypeConv for *mut T {
    fn get_type() -> Type {
        raw_ptr_ty(T::get_type().meta_kind())
    }
}

impl TypeConv for bool {
    fn get_type() -> Type {
        bool_ty()
    }
}

impl<T: TypeConv + ?Sized> TypeConv for &T {
    fn get_type() -> Type {
        ref_ty(PointeeInfo {
            size: T::get_size(),
            align: T::get_align(),
            inhabited: true,
            freeze: T::FREEZE,
            meta_kind: T::get_type().meta_kind(),
        })
    }
}

impl<T: TypeConv + ?Sized> TypeConv for &mut T {
    fn get_type() -> Type {
        ref_mut_ty(PointeeInfo {
            size: T::get_size(),
            align: T::get_align(),
            inhabited: true,
            freeze: T::FREEZE,
            meta_kind: T::get_type().meta_kind(),
        })
    }
}

impl<T: TypeConv, const N: usize> TypeConv for [T; N] {
    fn get_type() -> Type {
        array_ty(T::get_type(), N)
    }
}

impl TypeConv for () {
    fn get_type() -> Type {
        tuple_ty(&[], size(0), align(1))
    }
}
