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

/// The runtime metadata that can be stored in a wide pointer.
pub enum PointerMeta {}

/// A "pointer" is the thin pointer with optionally some metadata, making it a wide pointer.
/// This corresponds to the Rust raw pointer types, as well as references and boxes.
pub struct Pointer<Provenance> {
    pub thin_pointer: ThinPointer<Provenance>,
    pub metadata: Option<PointerMeta>,
}

/// The statically known kind of metadata stored with a pointer.
/// This has a one-to-one corresponcence with the variants of `Option<PointerMeta>`
pub enum PointerMetaKind {
    None,
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

impl PointerMetaKind {
    pub fn matches(self, meta: Option<PointerMeta>) -> bool {
        match (self, meta) {
            (PointerMetaKind::None, None) => true,
            _ => false,
        }
    }
}
```

## Pointee

We sometimes need information what it is that a pointer points to, this is captured in a "pointer type".
However, for unsized types the layout might depend on the pointer metadata, which gives rise to the "size strategy".

```rust
/// Describes what we know about data behind a pointer.
pub struct PointeeInfo {
    pub size: SizeStrategy,
    pub align: Align,
    pub inhabited: bool,
    pub freeze: bool,
    pub unpin: bool,
}

/// Describes how the size of the value can be determined.
pub enum SizeStrategy {
    /// The type is statically `Sized`.
    Sized(Size),
}

/// Stores all the information that we need to know about a pointer.
pub enum PtrType {
    Ref {
        /// Indicates a shared vs mutable reference.
        mutbl: Mutability,
        /// Describes what we know about the pointee.
        pointee: PointeeInfo,
    },
    Box {
        pointee: PointeeInfo,
    },
    Raw {
        /// Indicates what kind of metadata this pointer carries.
        meta_kind: PointerMetaKind,
    },
    FnPtr,
}
```


```rust
impl SizeStrategy {
    pub fn is_sized(self) -> bool {
        matches!(self, SizeStrategy::Sized(_))
    }

    /// Returns the size when the type must be statically sized.
    pub fn expect_sized(self, _msg: &str) -> Size {
        match self {
            SizeStrategy::Sized(size) => size,
            // TODO(UnsizedTypes): Panic on other variants
        }
    }

    /// Computes the dynamic size, but the caller must provide compatible metadata.
    pub fn compute(self, meta: Option<PointerMeta>) -> Size {
        match (self, meta) {
            (SizeStrategy::Sized(size), None) => size,
            _ => panic!("pointer meta data does not match type"),
        }
    }

    /// Returns the metadata kind which is needed to compute this strategy,
    /// i.e `self.meta_kind().matches(meta)` implies `self.compute(meta)` is well-defined.
    pub fn meta_kind(self) -> PointerMetaKind {
        match self {
            SizeStrategy::Sized(_) => PointerMetaKind::None,
        }
    }
}

impl PtrType {
    /// If this is a safe pointer, return the pointee information.
    pub fn safe_pointee(self) -> Option<PointeeInfo> {
        match self {
            PtrType::Ref { pointee, .. } | PtrType::Box { pointee, .. } => Some(pointee),
            PtrType::Raw { .. } | PtrType::FnPtr => None,
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

    pub fn meta_kind(self) -> PointerMetaKind {
        match self {
            PtrType::Ref { pointee, .. } | PtrType::Box { pointee, .. } => pointee.size.meta_kind(),
            PtrType::Raw { meta_kind, .. } => meta_kind,
            PtrType::FnPtr => PointerMetaKind::None,
        }
    }
}
```
