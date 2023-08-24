//! This allows you to convert Rust types to MiniRust types conveniently.

use crate::build::*;

/// Converts a Rust type to a MiniRust type.
/// Example usage: `let x: Type = <usize>::get_type();`
pub trait TypeConv {
    fn get_type() -> Type;
    fn get_align() -> Align;

    // Convenience methods, these should not be overridden.
    fn get_size() -> Size {
        Self::get_type().size::<DefaultTarget>()
    }

    fn get_ptype() -> PlaceType {
        PlaceType {
            ty: Self::get_type(),
            align: Self::get_align(),
        }
    }

    fn get_layout() -> Layout {
        Self::get_ptype().layout::<DefaultTarget>()
    }
}

macro_rules! type_conv_int_impl {
    ($ty:ty, $signed:expr, $size:expr, $align:expr) => {
        impl TypeConv for $ty {
            fn get_type() -> Type {
                Type::Int(IntType {
                    signed: $signed,
                    size: $size,
                })
            }
            fn get_align() -> Align {
                $align
            }
        }
    };
}

type_conv_int_impl!(u8, Unsigned, size(1), align(1));
type_conv_int_impl!(u16, Unsigned, size(2), align(2));
type_conv_int_impl!(u32, Unsigned, size(4), align(4));
type_conv_int_impl!(u64, Unsigned, size(8), align(8));
type_conv_int_impl!(u128, Unsigned, size(16), align(8));

type_conv_int_impl!(i8, Signed, size(1), align(1));
type_conv_int_impl!(i16, Signed, size(2), align(2));
type_conv_int_impl!(i32, Signed, size(4), align(4));
type_conv_int_impl!(i64, Signed, size(8), align(8));
type_conv_int_impl!(i128, Signed, size(16), align(8));

// We use `BasicMemory` to run a Program (see the `run` module),
// hence we have to use its PTR_SIZE for `usize` and `isize`.
type_conv_int_impl!(usize, Unsigned, DefaultTarget::PTR_SIZE, DefaultTarget::PTR_ALIGN);
type_conv_int_impl!(isize, Signed, DefaultTarget::PTR_SIZE, DefaultTarget::PTR_ALIGN);

impl<T: TypeConv> TypeConv for *const T {
    fn get_type() -> Type {
        raw_ptr_ty()
    }
    fn get_align() -> Align {
        DefaultTarget::PTR_ALIGN
    }
}

impl<T: TypeConv> TypeConv for *mut T {
    fn get_type() -> Type {
        raw_ptr_ty()
    }
    fn get_align() -> Align {
        DefaultTarget::PTR_ALIGN
    }
}

impl<T: TypeConv> TypeConv for &T {
    fn get_type() -> Type {
        ref_ty(T::get_layout())
    }
    fn get_align() -> Align {
        DefaultTarget::PTR_ALIGN
    }
}

impl<T: TypeConv> TypeConv for &mut T {
    fn get_type() -> Type {
        ref_mut_ty(T::get_layout())
    }
    fn get_align() -> Align {
        DefaultTarget::PTR_ALIGN
    }
}

impl TypeConv for bool {
    fn get_type() -> Type {
        bool_ty()
    }
    fn get_align() -> Align {
        align(1)
    }
}

impl<T: TypeConv, const N: usize> TypeConv for [T; N] {
    fn get_type() -> Type {
        array_ty(T::get_type(), N)
    }
    fn get_align() -> Align {
        T::get_align()
    }
}

impl TypeConv for () {
    fn get_type() -> Type {
        tuple_ty(&[], size(0))
    }
    fn get_align() -> Align {
        align(1)
    }
}
