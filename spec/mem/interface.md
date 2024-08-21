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

    /// Extra information for each stack frame.
    type FrameExtra;

    fn new() -> Self;

    /// Create a new allocation.
    /// The initial contents of the allocation are `AbstractByte::Uninit`.
    ///
    /// This is the only non-deterministic operation in the memory interface.
    fn allocate(&mut self, kind: AllocationKind, size: Size, align: Align) -> NdResult<Pointer<Self::Provenance>>;

    /// Remove an allocation.
    fn deallocate(&mut self, ptr: Pointer<Self::Provenance>, kind: AllocationKind, size: Size, align: Align) -> Result;

    /// Write some bytes to memory.
    fn store(&mut self, ptr: Pointer<Self::Provenance>, bytes: List<AbstractByte<Self::Provenance>>, align: Align) -> Result;

    /// Read some bytes from memory.
    ///
    /// Needs `&mut self` because in the aliasing model, reading changes the machine state.
    fn load(&mut self, ptr: Pointer<Self::Provenance>, len: Size, align: Align) -> Result<List<AbstractByte<Self::Provenance>>>;

    /// Test whether the given pointer is dereferenceable for the given size.
    fn dereferenceable(&self, ptr: Pointer<Self::Provenance>, len: Size) -> Result;

    /// A derived form of `dereferenceable` that works with a signed notion of "length".
    fn signed_dereferenceable(&self, ptr: Pointer<Self::Provenance>, len: Int) -> Result {
        if len > 0 {
            self.dereferenceable(ptr, Size::from_bytes(len).unwrap())
        } else {
            // Compute a pointer to the beginning of the range, and check `dereferenceable` from there.
            let begin_ptr = Pointer { addr: ptr.addr + len, ..ptr };
            // `ptr.addr + len` might be negative, but then `dereferenceable` will surely fail.
            self.dereferenceable(begin_ptr, Size::from_bytes(-len).unwrap())
        }
    }

    /// Retag the given pointer, which has the given type.
    /// `fn_entry` indicates whether this is one of the special retags that happen
    /// right at the top of each function.
    ///
    /// This must at least check that the pointer is `dereferenceable` for its size
    // (IOW, it cannot be more defined than the default implementation).
    ///
    /// Return the retagged pointer.
    fn retag_ptr(
        &mut self,
        _frame_extra: &mut Self::FrameExtra,
        ptr: Pointer<Self::Provenance>,
        ptr_type: PtrType,
        _fn_entry: bool,
    ) -> Result<Pointer<Self::Provenance>> {
        if let Some(layout) = ptr_type.safe_pointee() {
            self.dereferenceable(ptr, layout.size)?;
        }
        ret(ptr)
    }

    /// Create the extra information for a stack frame.
    fn new_call() -> Self::FrameExtra;

    /// Memory model hook invoked at the end of each function call.
    fn end_call(&mut self, _extra: Self::FrameExtra) -> Result { ret(()) }

    /// Check if there are any memory leaks.
    fn leak_check(&self) -> Result;
}
```

This is a very basic memory interface that is incomplete in at least the following ways:

* To represent concurrency, many operations need to take a "thread ID" and `load` and `store` need to take an [`Option<Ordering>`] (with `None` indicating non-atomic accesses).
* Maybe we want operations that can compare pointers without casting them to integers. Or else we decide only the address can matter for comparison.

[`Ordering`]: https://doc.rust-lang.org/nightly/core/sync/atomic/enum.Ordering.html
