# MiniRust pointers

One key question a memory model has to answer is *what is a pointer*.
It might seem like the answer is just "an integer of appropriate size", but [that is not the case][pointers-complicated] (as [more][pointers-complicated-2] and [more][pointers-complicated-3] discussion shows).
This becomes even more prominent with aliasing models such as [Stacked Borrows].
The memory model hence takes the stance that a pointer consists of the *address* (which truly is just an integer of appropriate size) and a *provenance*.
What exactly [provenance] *is* is up to the memory model.
As far as the interface is concerned, this is some opaque extra data that we carry around with our pointers and that places restrictions on which pointers may be used to do what when.

On top of this basic concept of a pointer, Rust also knows pointers with metadata (such as `*const [i32]`).
We therefore use the term *thin pointer* for what has been described above, and *pointer* for a pointer that optionally carries some metadata.

[pointers-complicated]: https://www.ralfj.de/blog/2018/07/24/pointers-and-bytes.html
[pointers-complicated-2]: https://www.ralfj.de/blog/2020/12/14/provenance.html
[pointers-complicated-3]: https://www.ralfj.de/blog/2022/04/11/provenance-exposed.html
[provenance]: https://github.com/rust-lang/unsafe-code-guidelines/blob/master/reference/src/glossary.md#pointer-provenance
[Stacked Borrows]: https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/stacked-borrows.md

## Pointer Types

```rust
/// An "address" is a location in memory. This corresponds to the actual
/// location in the real program.
/// We make it a mathematical integer, but of course it is bounded by the size
/// of the address space.
pub type Address = Int;

/// A "thin pointer" is an address together with its Provenance.
/// Provenance can be absent; those pointers are
/// invalid for all non-zero-sized accesses.
pub struct ThinPointer<Provenance> {
    pub addr: Address,
    pub provenance: Option<Provenance>,
}

/// A "pointer" is the thin pointer with optionally some metadata, making it a wide pointer.
/// This corresponds to the Rust raw pointer types, as well as references and boxes.
pub struct Pointer<Provenance> {
    pub thin_pointer: ThinPointer<Provenance>,
    pub metadata: Option<PointerMeta>,
}

/// The metadata that can be stored in a wide pointer.
pub enum PointerMeta {
    ElementCount(Int),
}

impl<Provenance> ThinPointer<Provenance> {
    /// Offsets a pointer in bytes using wrapping arithmetic.
    /// This does not check whether the pointer is still in-bounds of its allocation.
    pub fn wrapping_offset<T: Target>(self, offset: Int) -> Self {
        let addr = self.addr + offset;
        let addr = addr.bring_in_bounds(Unsigned, T::PTR_SIZE);
        ThinPointer { addr, ..self }
    }

    pub fn widen(self, metadata: Option<PointerMeta>) -> Pointer<Provenance> {
        Pointer {
            thin_pointer: self,
            metadata,
        }
    }
}
```

## Layout

We sometimes need information what it is that a pointer points to, this is captured in a "pointer type".
However, for unsized types the layout might depend on the pointer metadata, which gives rise to the "size strategy".

```rust
/// A "layout" describes what we know about data behind a pointer.
pub struct Layout {
    pub size: SizeStrategy,
    pub align: Align,
    pub inhabited: bool,
}

/// This describes how the size of the value can be determined.
pub enum SizeStrategy {
    /// The type is statically `Sized`.
    Sized(Size),

    /// The size of the type is given by `min_size + element_size * len`,
    /// where `len` is found in the wide pointer metadata.
    SliceTail {
        min_size: Size,
        element_size: Size,
    },
}

impl SizeStrategy {
    pub fn is_sized(self) -> bool {
        matches!(self, SizeStrategy::Sized(_))
    }

    /// Returns the size when the type must be statically sized
    pub fn unwrap_size(self) -> Size {
        match self {
            SizeStrategy::Sized(size) => size,
            _ => panic!("Expected a sized type"),
        }
    }

    pub fn resolve(self, meta: Option<PointerMeta>) -> Size {
        match (self, meta) {
            (SizeStrategy::Sized(size), None) => size,
            (SizeStrategy::SliceTail { min_size, element_size }, Some(PointerMeta::ElementCount(num))) => {
                min_size + element_size * num
            }
            _ => panic!("Pointer meta data does not match type"),
        }
    }
}

pub enum PtrType {
    Ref {
        /// Indicates a shared vs mutable reference.
        /// FIXME: also indicate presence of `UnsafeCell`.
        mutbl: Mutability,
        /// We only need to know the layout of the pointee, not the full type.
        /// (This also means we have a finite representation even when the Rust type is recursive.)
        pointee: Layout,
    },
    Box {
        pointee: Layout,
    },
    Raw {
        /// This is not a safe pointer, but we still need to know what kind of metadata is needed.
        pointee: Layout,
    },
    FnPtr,
}

impl PtrType {
    pub fn pointee(self) -> Option<Layout> {
        match self {
            PtrType::Ref { pointee, .. } | PtrType::Box { pointee, .. } | PtrType::Raw { pointee, .. } => Some(pointee),
            PtrType::FnPtr => None,
        }
    }

    /// If this is a safe pointer, only then return the pointee layout.
    pub fn safe_pointee(self) -> Option<Layout> {
        match self {
            PtrType::Raw { .. } => None,
            _ => self.pointee(),
        }
    }

    pub fn matches_meta(self, meta: Option<PointerMeta>) -> bool {
        let pointee_size = self.pointee().map(|l| l.size);
        match (pointee_size, meta) {
            (None, None) => true,
            (Some(SizeStrategy::Sized(_)), None) => true,
            (Some(SizeStrategy::SliceTail { .. }), Some(PointerMeta::ElementCount(_))) => true,
            _ => false,
        }
    }

    pub fn addr_valid(self, addr: Address) -> bool {
        if let Some(layout) = self.safe_pointee() {
            // Safe addresses need to be non-null, aligned, and not point to an uninhabited type.
            // (Think: uninhabited types have impossible alignment.)
            addr != 0 && layout.align.is_aligned(addr) && layout.inhabited
        } else {
            true
        }
    }
}
```
