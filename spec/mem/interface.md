# MiniRust Memory Interface

The purpose of this document is to describe the interface between a MiniRust program and memory.

The interface shown below already makes several key decisions.
It is not intended to be able to support *any imaginable* memory model, but rather start the process of reducing the design space of what we consider a "reasonable" memory model for Rust.
For example, it explicitly acknowledges that pointers are not just integers and that [uninitialized memory is special][uninit] (both are true for C and C++ as well but you have to read the standard very careful, and consult [defect report responses](http://www.open-std.org/jtc1/sc22/wg14/www/docs/dr_260.htm), to see this).
Another key property of the interface presented below is that it is *untyped*.
This implies that in MiniRust, *operations are typed, but memory is not* - a key difference to C and C++ with their type-based strict aliasing rules.

[uninit]: https://www.ralfj.de/blog/2019/07/14/uninit.html

## Abstract Bytes

The unit of communication between the memory model and the rest of the program is a *byte*.
To distinguish our MiniRust bytes from `u8`, we will call them "abstract bytes".
Abstract bytes differ from `u8` to support representing uninitialized Memory and to support maintaining pointer provenance when pointers are stored in memory.
We define the `AbstractByte` type as follows, where `Provenance` will later be instantiated with the `Memory::Provenance` associated type.

```rust
pub enum AbstractByte<Provenance> {
    /// An uninitialized byte.
    Uninit,
    /// An initialized byte, optionally with some provenance (if it is encoding a pointer).
    Init(u8, Option<Provenance>),
}

impl<Provenance> AbstractByte<Provenance> {
    pub fn data(self) -> Option<u8> {
        match self {
            AbstractByte::Uninit => None,
            AbstractByte::Init(data, _) => Some(data),
        }
    }

    pub fn provenance(self) -> Option<Provenance> {
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
/// The "kind" of an allocation is used to distinguish, for instance, stack from heap memory.
pub enum AllocationKind {
    /// Memory for a stack variable.
    Stack,
    /// Memory allocated with the AM heap intrinsics.
    Heap,
    /// Memory for a global variable.
    Global,
    /// Memory for a function.
    Function,
}

/// *Note*: All memory operations can be non-deterministic, which means that
/// executing the same operation on the same memory can have different results.
/// We also let read operations potentially mutate memory (they actually can
/// change the current state in concurrent memory models and in Stacked Borrows).
pub trait Memory {
    /// The target information.
    /// This doesn't really belong to the memory, but avoids having to quantify over both
    /// memory and target everywhere.
    type T: Target;

    /// The type of pointer provenance.
    type Provenance;

    fn new() -> Self;

    /// Create a new allocation.
    /// The initial contents of the allocation are `AbstractByte::Uninit`.
    fn allocate(&mut self, kind: AllocationKind, size: Size, align: Align) -> NdResult<Pointer<Self::Provenance>>;

    /// Remove an allocation.
    fn deallocate(&mut self, ptr: Pointer<Self::Provenance>, kind: AllocationKind, size: Size, align: Align) -> Result;

    /// Write some bytes to memory.
    fn store(&mut self, ptr: Pointer<Self::Provenance>, bytes: List<AbstractByte<Self::Provenance>>, align: Align) -> Result;

    /// Read some bytes from memory.
    fn load(&mut self, ptr: Pointer<Self::Provenance>, len: Size, align: Align) -> Result<List<AbstractByte<Self::Provenance>>>;

    /// Test whether the given pointer is dereferenceable for the given layout, including an alignment check.
    /// Raises UB if that is not the case. Must always raise UB for uninhabited layouts.
    fn dereferenceable(&self, ptr: Pointer<Self::Provenance>, layout: Layout) -> Result;

    /// Retag the given pointer, which has the given type.
    /// `fn_entry` indicates whether this is one of the special retags that happen
    /// right at the top of each function.
    /// 
    /// This must *at least* ensure that if the `ptr_type` carries a layout, then
    /// the pointer is dereferenceable at that layout.
    ///
    /// Return the retagged pointer.
    fn retag_ptr(&mut self, ptr: Pointer<Self::Provenance>, ptr_type: PtrType, fn_entry: bool) -> Result<Pointer<Self::Provenance>>;
}
```

This is a very basic memory interface that is incomplete in at least the following ways:

* To represent concurrency, many operations need to take a "thread ID" and `load` and `store` need to take an [`Option<Ordering>`] (with `None` indicating non-atomic accesses).
* Maybe we want operations that can compare pointers without casting them to integers. Or else we decide only the address can matter for comparison.

[`Ordering`]: https://doc.rust-lang.org/nightly/core/sync/atomic/enum.Ordering.html


```rust
impl<Provenance> Pointer<Provenance> {
    /// Calculates the offset from a pointer in bytes using wrapping arithmetic.
    /// This does not check whether the pointer is still in-bounds of its allocation.
    pub fn wrapping_offset<M: Memory<Provenance=Provenance>>(self, offset: Int) -> Self {
        let offset = offset.modulo(Signed, M::T::PTR_SIZE);
        let addr = self.addr + offset;
        let addr = addr.modulo(Unsigned, M::T::PTR_SIZE);
        Pointer { addr, ..self }
    }
}
```
