# MiniRust Memory Interface

The purpose of this document is to describe the interface between a MiniRust program and memory.

The interface shown below already makes several key decisions.
It is not intended to be able to support *any imaginable* memory model, but rather start the process of reducing the design space of what we consider a "reasonable" memory model for Rust.
For example, it explicitly acknowledges that pointers are not just integers and that [uninitialized memory is special][uninit] (both are true for C and C++ as well but you have to read the standard very careful, and consult [defect report responses](http://www.open-std.org/jtc1/sc22/wg14/www/docs/dr_260.htm), to see this).
Another key property of the interface presented below is that it is *untyped*.
This implies that in MiniRust, *operations are typed, but memory is not* - a key difference to C and C++ with their type-based strict aliasing rules.

[uninit]: https://www.ralfj.de/blog/2019/07/14/uninit.html

## Pointers

One key question a memory model has to answer is *what is a pointer*.
It might seem like the answer is just "an integer of appropriate size", but [that is not the case][pointers-complicated] (as [more][pointers-complicated-2] and [more][pointers-complicated-3] discussion shows).
This becomes even more prominent with aliasing models such as [Stacked Borrows].
The memory model hence takes the stance that a pointer consists of the *address* (which truly is just an integer of appropriate size) and a *provenance*.
What exactly [provenance] *is* is up to the memory model.
As far as the interface is concerned, this is some opaque extra data that we carry around with our pointers and that places restrictions on which pointers may be used to do what when.

[pointers-complicated]: https://www.ralfj.de/blog/2018/07/24/pointers-and-bytes.html
[pointers-complicated-2]: https://www.ralfj.de/blog/2020/12/14/provenance.html
[pointers-complicated-3]: https://www.ralfj.de/blog/2022/04/11/provenance-exposed.html
[provenance]: https://github.com/rust-lang/unsafe-code-guidelines/blob/master/reference/src/glossary.md#pointer-provenance
[Stacked Borrows]: https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/stacked-borrows.md

## Abstract Bytes

The unit of communication between the memory model and the rest of the program is a *byte*.
To distinguish our MiniRust bytes from `u8`, we will call them "abstract bytes".
Abstract bytes differ from `u8` to support representing uninitialized Memory and to support maintaining pointer provenance when pointers are stored in memory.
We define the `AbstractByte` type as follows, where `Provenance` will later be instantiated with the `Memory::Provenance` associated type.

```rust
#[derive(PartialEq, Eq)]
pub enum AbstractByte<Provenance> {
    /// An uninitialized byte.
    Uninit,
    /// An initialized byte, optionally with some provenance (if it is encoding a pointer).
    Init(u8, Option<Provenance>),
}

impl<Provenance> AbstractByte<Provenance> {
    fn data(self) -> Option<u8> {
        match self {
            AbstractByte::Uninit => None,
            AbstractByte::Init(data, _) => Some(data),
        }
    }

    fn provenance(self) -> Option<Provenance> {
        match self {
            AbstractByte::Uninit => None,
            AbstractByte::Init(_, provenance) => provenance,
        }
    }
}
```

## Memory interface

The MiniRust memory interface is described by the following (not-yet-complete) trait definition:

```rust
/// An "address" is a location in memory. This corresponds to the actual
/// location in the real program.
/// We make it a mathematical integer, but of course it is bounded by the size
/// of the address space.
type Address = BigInt;

/// A "pointer" is an address together with its Provenance.
/// Provenance can be absent; those pointers are
/// invalid for all non-zero-sized accesses.
#[derive(PartialEq, Eq)]
pub struct Pointer<Provenance> {
    addr: Address,
    provenance: Option<Provenance>,
}

/// *Note*: All memory operations can be non-deterministic, which means that
/// executing the same operation on the same memory can have different results.
/// We also let read operations potentially mutate memory (they actually can
/// change the current state in concurrent memory models and in Stacked Borrows).
pub trait Memory: Sized {
    /// The type of pointer provenance.
    type Provenance: Eq;

    /// The size of a pointer.
    const PTR_SIZE: Size;

    /// The endianess used for encoding multi-byte integer values (and pointers).
    const ENDIANNESS: Endianness;

    /// Create a new allocation.
    /// The initial contents of the allocation are `AbstractByte::Uninit`.
    fn allocate(&mut self, size: Size, align: Align) -> NdResult<Pointer<Self::Provenance>>;

    /// Remove an allocation.
    fn deallocate(&mut self, ptr: Pointer<Self::Provenance>, size: Size, align: Align) -> Result;

    /// Write some bytes to memory.
    fn store(&mut self, ptr: Pointer<Self::Provenance>, bytes: List<AbstractByte<Self::Provenance>>, align: Align) -> Result;

    /// Read some bytes from memory.
    fn load(&mut self, ptr: Pointer<Self::Provenance>, len: Size, align: Align) -> Result<List<AbstractByte<Self::Provenance>>>;

    /// Test whether the given pointer is dereferenceable for the given size and alignment.
    /// Raises UB if that is not the case.
    /// Note that a successful read/write/deallocate implies that the pointer
    /// was dereferenceable before that operation (but not vice versa).
    fn dereferenceable(&self, ptr: Pointer<Self::Provenance>, size: Size, align: Align) -> Result;

    /// Retag the given pointer, which has the given type.
    /// `fn_entry` indicates whether this is one of the special retags that happen
    /// right at the top of each function.
    /// FIXME: Referencing `PtrType` here feels like a layering violation, but OTOH
    /// also seems better than just outright duplicating that type.
    ///
    /// Return the retagged pointer.
    fn retag_ptr(&mut self, ptr: Pointer<Self::Provenance>, ptr_type: lang::PtrType, fn_entry: bool) -> Result<Pointer<Self::Provenance>>;
}
```

This is a very basic memory interface that is incomplete in at least the following ways:

* We need to add support for [casting pointers to integers](https://doc.rust-lang.org/nightly/std/primitive.pointer.html#method.expose_addr) and [back](https://doc.rust-lang.org/nightly/std/ptr/fn.from_exposed_addr.html).
* To represent concurrency, many operations need to take a "thread ID" and `load` and `store` need to take an [`Option<Ordering>`] (with `None` indicating non-atomic accesses).
* To represent [Stacked Borrows], there needs to be a "retag" operation, and that one might have to be "lightly typed" (to care about `UnsafeCell`).
* Maybe we want operations that can compare pointers without casting them to integers.

[`Ordering`]: https://doc.rust-lang.org/nightly/core/sync/atomic/enum.Ordering.html
