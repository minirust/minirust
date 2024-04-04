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

Note that for now, we make the exact offsets of each field part of the type.
As always, this definition is incomplete.
In the future, we might want to separate a type from its layout, and consider these separate components -- we will have to see what works best.

```rust
/// The types of MiniRust.
pub enum Type {
    Int(IntType),
    Bool,
    Ptr(PtrType),
    /// "Tuple" is used for all heterogeneous types, i.e., both Rust tuples and structs.
    Tuple {
        /// Fields must not overlap.
        fields: Fields,
        /// The total size of the tuple can indicate trailing padding.
        /// Must be large enough to contain all fields.
        size: Size,
        /// Total alignment of the tuple. Due to `repr(packed)` and `repr(align)`,
        /// this is independent of the fields' alignment.
        align: Align,
    },
    Array {
        #[specr::indirection]
        elem: Type,
        count: Int,
        // TODO: store whether this is a (SIMD) vector, and something about alignment?
    },
    Union {
        /// Fields *may* overlap. Fields only exist for field access place projections,
        /// they are irrelevant for the representation relation.
        fields: Fields,
        /// A union can be split into multiple "chunks", where only the data inside those chunks is
        /// preserved, and data between chunks is lost (like padding in a struct).
        /// This is necessary to model the behavior of some `repr(C)` unions, see
        /// <https://github.com/rust-lang/unsafe-code-guidelines/issues/156> for details.
        chunks: List<(Offset, Size)>,
        /// The total size of the union, can indicate padding after the last chunk.
        size: Size,
        /// Total alignment of the union. Due to `repr(packed)` and `repr(align)`,
        /// this is independent of the fields' alignment.
        align: Align,
    },
    Enum {
        /// The map variants, each identified by a discriminant. Each variant is given by a type and its
        /// tag description. All variants are thought to "start at offset 0"; if the
        /// discriminant is encoded as an explicit tag, then that will be put into the
        /// padding of the active variant. (This means it is *not* safe to hand out mutable
        /// references to a variant at that type, as then the tag might be overwritten!)
        /// The Rust type `!` is encoded as an `Enum` with an empty list of variants.
        variants: Map<Int, Variant>,
        /// The `IntType` for the discriminant. This is used for the type of
        /// `GetDiscriminant` and `SetDiscriminant`. It is entirely independent of how
        /// the discriminant is represented in memory (the "tag").
        discriminant_ty: IntType,
        /// The decision tree to decode the discriminant from the tag at runtime.
        discriminator: Discriminator,
        /// The total size of the enum can indicate trailing padding.
        /// Must be large enough to contain all variants.
        size: Size,
        /// Total alignment of the enum. Due to `repr(packed)` and `repr(align)`,
        /// this is independent of the fields' alignment.
        align: Align,
    },
}

pub struct IntType {
    pub signed: Signedness,
    pub size: Size,
}

pub type Fields = List<(Offset, Type)>;

pub struct Variant {
    /// The actual type of the variant.
    pub ty: Type,
    /// The information on where to store which values to write the tag.
    /// MUST NOT touch any bytes written by the actual type of the variant and vice
    /// versa. This is because we allow references/pointers to (enum) fields which
    /// should be able to dereference without having to deal with the tag.
    pub tagger: Map<Offset, (IntType, Int)>,
}

/// The decision tree that computes the discriminant out of the tag for a specific
/// enum type.
pub enum Discriminator {
    /// We know the discriminant.
    Known(Int),
    /// Tag decoding failed, there is no valid discriminant.
    Invalid,
    /// We don't know the discriminant, so we branch on the value of a specific value.
    Branch {
        offset: Offset,
        value_type: IntType,
        #[specr::indirection]
        fallback: Discriminator,
        /// An left-inclusive right-exclusive range of values that map to some Discriminator.
        children: Map<(Int, Int), Discriminator>,
    },
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

    pub fn align<T: Target>(self) -> Align {
        use Type::*;
        match self {
            Int(int_type) => {
                let size = int_type.size.bytes();
                // The size is a power of two, so we can use it as alignment.
                let natural_align = Align::from_bytes(size).unwrap();
                // Integer alignment is capped by the target.
                natural_align.min(T::INT_MAX_ALIGN)
            }
            Bool => Align::ONE,
            Ptr(_) => T::PTR_ALIGN,
            Tuple { align, .. } | Union { align, .. } | Enum { align, .. } => align,
            Array { elem, .. } => elem.align::<T>(),
        }
    }

    pub fn inhabited(self) -> bool {
        use Type::*;
        match self {
            Int(..) | Bool | Ptr(PtrType::Raw { .. }) | Ptr(PtrType::FnPtr(_)) => true,
            Ptr(PtrType::Ref { pointee, .. } | PtrType::Box { pointee }) => pointee.inhabited,
            Tuple { fields, .. } => fields.all(|(_offset, ty)| ty.inhabited()),
            Array { elem, count } => count == 0 || elem.inhabited(),
            Union { .. } => true,
            Enum { variants, .. } => variants.values().any(|variant| variant.ty.inhabited()),
        }
    }

    pub fn layout<T: Target>(self) -> Layout {
        Layout {
            size: self.size::<T>(),
            align: self.align::<T>(),
            inhabited: self.inhabited(),
        }
    }
}
```

## Integer type convenience functions

```rust
impl IntType {
    pub fn can_represent(&self, i: Int) -> bool {
        i.in_bounds(self.signed, self.size)
    }

    pub fn bring_in_bounds(&self, i: Int) -> Int {
        i.modulo(self.signed, self.size)
    }
}
```
