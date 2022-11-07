# MiniRust types

This file defines the types of MiniRust.
Note that MiniRust types play a somewhat different role than Rust types:
every Rust type corresponds to a MiniRust type, but MiniRust types are merely annotated at various operations to define how [values](values.md) are represented in memory.
Basically, they only define a (de)serialization format -- the **representation relation**, define by an "encode" function to turn values into byte lists, and a "decode" function for the opposite operation.
In particular, MiniRust is by design *not type-safe*.
However, the representation relation is a key part of the language, since it forms the interface between the low-level and high-level view of data, between lists of (abstract) bytes and [values](values.md).
For pointer types (references and raw pointers), we types also contain a "mutability", which does not affect the representation relation but can be relevant for the aliasing rules.
(We might want to organize this differently in the future, and remove mutability from types.)

MiniRust has types `Type` for values, and `PlaceType` for places.
Place types combine a value type with an alignment; places of that type are guaranteed to be suitably aligned.
Rust types correspond to place types.
This distinction allows us to elegantly encode `repr(packed)` and `repr(align)` by varying the `align` field of the place type.
It also elegantly avoids having to define a function that computes the alignment for any `Type` -- that is almost entirely target-dependent anyway, and not at all related to how values of that type get (de)serialized.

Note that for now, we make the exact offsets of each field part of the type.
As always, this definition is incomplete.
In the future, we might want to separate a type from its layout, and consider these separate components -- we will have to see what works best.

```rust
/// A "layout" describes the shape of data in memory.
struct Layout {
    size: Size,
    align: Align,
    inhabited: bool,
}

/// "Value" types -- these have a size, but not an alignment.
enum Type {
    Int(IntType),
    Bool,
    Pointer(PtrType),
    /// "Tuple" is used for all heterogeneous types, i.e., both Rust tuples and structs.
    Tuple {
        /// Fields must not overlap.
        fields: Fields,
        /// The total size of the type can indicate trailing padding.
        /// Must be large enough to contain all fields.
        size: Size,
    },
    Array {
        elem: Type,
        count: BigInt,
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

pub enum PtrType {
    Ref {
        /// Indicates a shared vs mutable reference.
        /// FIXME: also indicate presence of `UnsafeCell`.
        mutbl: Mutability,
        /// We only need to know the layout of the pointee.
        /// (This also means we have a finite representation even when the Rust type is recursive.)
        pointee: Layout,
    },
    Box {
        pointee: Layout,
    },
    Raw {
        /// Raw pointer layout is relevant for Stacked Borrows retagging.
        /// TODO: I hope we can remove this in the future.
        pointee: Layout,
    },
}

struct IntType {
    signed: Signedness,
    size: Size,
}

type Fields = List<(Size, Type)>; // (offset, type) pair for each field

/// We leave the details of enum tags to the future.
/// (We might want to extend the "variants" field of `Enum` to also have a
/// discriminant for each variant. We will see.)
enum TagEncoding { /* ... */ }

/// "Place" types are laid out in memory and thus also have an alignment requirement.
struct PlaceType {
    ty: Type,
    align: Align,
}
```

Note that references have no lifetime, since the lifetime is irrelevant for their representation in memory!
They *do* have a mutability since that is (or will be) relevant for the memory model.

## Layout of a type

Here we define how to compute the size and other layout properties of a type.

```rust
impl Type {
    fn size<M: Memory>(self) -> Size {
        use Type::*;
        match self {
            Int(int_type) => int_type.size,
            Bool => Size::from_bytes(1).unwrap(),
            Pointer(_) => M::PTR_SIZE,
            Tuple { size, .. } | Union { size, .. } | Enum { size, .. } => size,
            Array { elem, count } => elem.size::<M>() * count,
        }
    }

    fn inhabited(self) -> bool {
        use Type::*;
        match self {
            Int(..) | Bool | Pointer(PtrType::Raw { .. }) => true,
            Pointer(PtrType::Ref { pointee, .. } | PtrType::Box { pointee }) => pointee.inhabited,
            Tuple { fields, .. } => fields.iter().all(|ty| ty.inhabited()),
            Array { elem, count } => count == 0 || elem.inhabited(),
            Union { .. } => true,
            Enum { variants, .. } => variants.iter().any(|ty| ty.inhabited()),
        }
    }
}

impl PlaceType {
    fn new(ty: Type, align: Align) -> Self {
        PlaceType { ty, align }
    }

    fn layout<M: Memory>(self) -> Layout {
        Layout {
            size: self.ty.size::<M>(),
            align: self.align,
            inhabited: self.ty.inhabited(),
        }
    }
}
```
