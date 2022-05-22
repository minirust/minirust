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
[Stacked Borrows]: stacked-borrows.md

## Abstract Bytes

The unit of communication between the memory model and the rest of the program is a *byte*.
To distinguish our MiniRust bytes from `u8`, we will call them "abstract bytes".
Abstract bytes differ from `u8` to support representing uninitialized Memory and to support maintaining pointer provenance when pointers are stored in memory.
We define the `AbstractByte` type as follows, where `Provenance` will later be instantiated with the `Memory::Provenance` associated type.

```rust
enum AbstractByte<Provenance> {
    /// An uninitialized byte.
    Uninit,
    /// The "normal" case: a (frozen, initialized) integer in `0..256`.
    Raw(u8),
    /// One byte of a pointer.
    Ptr(u8, Provenance),
}

impl AbstractByte<P> {
    fn data(self) -> Option<u8> {
        match self {
            Uninit => None,
            Raw(data) | Ptr(data, _) => Some(data),
        }
    }

    fn provenance(self) -> Option<Provenance> {
        match self {
            Uninit | Raw(_) => None,
            Ptr(_, provenance) => Some(provenance),
        }
    }
}
```

## Memory interface

The MiniRust memory interface is described by the following (not-yet-complete) trait definition:

```rust
/// All operations are fallible, so they return `Result`.  If they fail, that
/// means the program caused UB. What exactly the `UndefinedBehavior` type is
/// does not matter here.
type Result<T=()> = std::result::Result<T, UndefinedBehavior>;

/// A "pointer" is an address (`u64` should be large enough for all targets... TM)
/// together with its Provenance.
type Address = u64;
type Pointer<Provenance> = (Address, Provenance);

/// *Note*: All memory operations can be non-deterministic, which means that
/// executing the same operation on the same memory can have different results.
/// We also let read operations potentially mutate memory (they actually can
/// change the current state in concurrent memory models and in Stacked Borrows).
trait MemoryInterface {
    /// The type of pointer provenance.
    type Provenance;

    /// We use `Self::Pointer` as notation for `Pointer<Self::Provenance>`,
    /// and `Self::AbstractByte` as notation for `AbstractByte<Self::Provenance>`.
    type Pointer = Pointer<Self::Provenance>;
    type AbstractByte = AbstractByte<Self::Provenance>;

    /// The provenance of an "invalid" pointer that cannot be used for (non-ZST)
    /// memory accesses.
    const INVALID_PROVENANCE: Provenance;

    /// Create a new allocation.
    /// The initial contents of the allocation are `AbstractByte::Uninit`.
    fn allocate(&mut self, size: Size, align: Align) -> Result<Self::Pointer>;

    /// Remove an allocation.
    fn deallocate(&mut self, ptr: Self::Pointer, size: Size, align: Align) -> Result;

    /// Write some bytes to memory.
    fn write(&mut self, ptr: Self::Pointer, bytes: Vec<Self::AbstractByte>) -> Result;

    /// Read some bytes from memory.
    fn read(&mut self, ptr: Self::Pointer, len: Size) -> Result<Vec<Self::AbstractByte>>;

    /// Test whether the given pointer is dereferencable for the given size and alignment.
    /// Raises UB if that is not the case.
    /// Note that a successful read/write/deallocate implies that the pointer
    /// was dereferencable before that operation (but not vice versa).
    fn dereferencable(&self, ptr: Self::Pointer, size: Size, align: Align) -> Result;
}
```

This is a very basic memory interface that is incomplete in at least the following ways:

* We need to add support for [casting pointers to integers](https://doc.rust-lang.org/nightly/std/primitive.pointer.html#method.expose_addr) and [back](https://doc.rust-lang.org/nightly/std/ptr/fn.from_exposed_addr.html).
* To represent concurrency, many operations need to take a "thread ID" and `read` and `write` need to take an [`Ordering`].
* To represent [Stacked Borrows], there needs to be a "retag" operation, and that one might have to be "lightly typed" (to care about `UnsafeCell`).
* Maybe we want operations that can compare pointers without casting them to integers.

[`Ordering`]: https://doc.rust-lang.org/nightly/core/sync/atomic/enum.Ordering.html
