# MiniRust types

This file defines the types of MiniRust.
Note that MiniRust types play a somewhat different role than Rust types:
every Rust type corresponds to a MiniRust type, but MiniRust types mostly just serve to define how [values](values.md) are represented in memory.
Basically, they define a (de)serialization format -- the **representation relation**, defined by an "encode" function to turn values into byte lists, and a "decode" function for the opposite operation.
In particular, MiniRust is by design *not type-safe*.
However, the representation relation is a key part of the language, since it forms the interface between the low-level and high-level view of data, between lists of (abstract) bytes and values.

That said, types do have a little more information than required for the representation relation.
For pointer types (references and raw pointers), types also contain a "mutability", which does not affect the representation relation but can be relevant for the aliasing rules.
(We might want to organize this differently in the future, and remove mutability from types.)
Union types know the types of their fields solely to support union field place projections.

MiniRust has types `Type` for values, and `PlaceType` for places.
Place types combine a value type with an alignment; places of that type are guaranteed to be suitably aligned.
Rust types correspond to place types.
This distinction allows us to elegantly encode `repr(packed)` and `repr(align)` by varying the `align` field of the place type.
It also elegantly avoids having to define a function that computes the alignment for any `Type` -- that is almost entirely target-dependent anyway, and not at all related to how values of that type get (de)serialized.

Note that for now, we make the exact offsets of each field part of the type.
As always, this definition is incomplete.
In the future, we might want to separate a type from its layout, and consider these separate components -- we will have to see what works best.

```rust
/// "Value" types -- these have a size, but not an alignment.
pub enum Type {
    Int(IntType),
    Bool,
    Ptr(PtrType),
    /// "Tuple" is used for all heterogeneous types, i.e., both Rust tuples and structs.
    Tuple {
        /// Fields must not overlap.
        fields: Fields,
        /// The total size of the type can indicate trailing padding.
        /// Must be large enough to contain all fields.
        size: Size,
    },
    Array {
        #[specr::indirection]
        elem: Type,
        count: Int,
    },
    Union {
        /// Fields *may* overlap. Fields only exist for field access place projections,
        /// they are irrelevant for the representation relation.
        fields: Fields,
        /// A union can be split into multiple "chunks", where only the data inside those chunks is
        /// preserved, and data between chunks is lost (like padding in a struct).
        /// This is necessary to model the behavior of some `repr(C)` unions, see
        /// <https://github.com/rust-lang/unsafe-code-guidelines/issues/156> for details.
        chunks: List<(Size, Size)>, // (offset, length) for each chunk.
        /// The total size of the union, can indicate padding after the last chunk.
        size: Size,
    },
    Enum {
        /// Each variant is given by a type. All types are thought to "start at offset 0";
        /// if the discriminant is encoded as an explicit tag, then that will be put
        /// into the padding of the active variant. (This means it is *not* safe to hand
        /// out mutable references to a variant at that type, as then the tag might be
        /// overwritten!)
        /// The Rust type `!` is encoded as an `Enum` with an empty list of variants.
        variants: List<Type>,
        /// This contains all the tricky details of how to encode the active variant
        /// at runtime.
        tag_encoding: TagEncoding,
        /// The total size of the type can indicate trailing padding.
        /// Must be large enough to contain all variants.
        size: Size,
    },
}



pub struct IntType {
    pub signed: Signedness,
    pub size: Size,
}

pub type Fields = List<(Size, Type)>; // (offset, type) pair for each field

/// We leave the details of enum tags to the future.
/// (We might want to extend the "variants" field of `Enum` to also have a
/// discriminant for each variant. We will see.)
pub enum TagEncoding { /* ... */ }

/// "Place" types are laid out in memory and thus also have an alignment requirement.
pub struct PlaceType {
    pub ty: Type,
    pub align: Align,
}
```

Note that references have no lifetime, since the lifetime is irrelevant for their representation in memory!
They *do* have a mutability since that is (or will be) relevant for the memory model.

## Layout of a type

Here we define how to compute the size and other layout properties of a type.

```rust
impl Type {
    pub fn size<T: Target>(self) -> Size {
        use Type::*;
        match self {
            Int(int_type) => int_type.size,
            Bool => Size::from_bytes_const(1),
            Ptr(_) => T::PTR_SIZE,
            Tuple { size, .. } | Union { size, .. } | Enum { size, .. } => size,
            Array { elem, count } => elem.size::<T>() * count,
        }
    }

    pub fn inhabited(self) -> bool {
        use Type::*;
        match self {
            Int(..) | Bool | Ptr(PtrType::Raw { .. }) | Ptr(PtrType::FnPtr) => true,
            Ptr(PtrType::Ref { pointee, .. } | PtrType::Box { pointee }) => pointee.inhabited,
            Tuple { fields, .. } => fields.all(|(_offset, ty)| ty.inhabited()),
            Array { elem, count } => count == 0 || elem.inhabited(),
            Union { .. } => true,
            Enum { variants, .. } => variants.any(|ty| ty.inhabited()),
        }
    }
}

impl PlaceType {
    pub fn new(ty: Type, align: Align) -> Self {
        PlaceType { ty, align }
    }

    pub fn layout<T: Target>(self) -> Layout {
        Layout {
            size: self.ty.size::<T>(),
            align: self.align,
            inhabited: self.ty.inhabited(),
        }
    }
}
```
