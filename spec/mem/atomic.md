# MiniRust atomic memory

This is a wrapper for a memory that distinguishes between non-atomic and atomic memory accesses.

```rust
pub struct AtomicMemory<M: Memory> {
    memory: M,
}

/// The different kinds of atomicity.
pub enum Atomicity {
    /// A sequentially consistent atomic access.
    Atomic,

    /// A non-atomic memory access.
    None,
}

impl<M: Memory> AtomicMemory<M> {
    pub fn new() -> Self {
        Self { memory: M::new() }
    }

    /// Create a new allocation.
    /// The initial contents of the allocation are `AbstractByte::Uninit`.
    pub fn allocate(&mut self, size: Size, align: Align) -> NdResult<Pointer<M::Provenance>> {
        self.memory.allocate(size, align)
    }

    /// Remove an allocation.
    pub fn deallocate(&mut self, ptr: Pointer<M::Provenance>, size: Size, align: Align) -> Result {
        self.memory.deallocate(ptr, size, align)
    }

    /// Write some bytes to memory.
    pub fn store(&mut self, _atomicity: Atomicity, ptr: Pointer<M::Provenance>, bytes: List<AbstractByte<M::Provenance>>, align: Align) -> Result {
        self.memory.store(ptr, bytes, align)
    }

    /// Read some bytes from memory.
    pub fn load(&mut self, _atomicity: Atomicity, ptr: Pointer<M::Provenance>, len: Size, align: Align) -> Result<List<AbstractByte<M::Provenance>>> {
        self.memory.load(ptr, len, align)
    }

    /// Test whether the given pointer is dereferenceable for the given size and alignment.
    /// Raises UB if that is not the case.
    /// Note that a successful read/write/deallocate implies that the pointer
    /// was dereferenceable before that operation (but not vice versa).
    pub fn dereferenceable(&self, ptr: Pointer<M::Provenance>, size: Size, align: Align) -> Result {
        self.memory.dereferenceable(ptr, size, align)
    }

    /// Return the retagged pointer.
    pub fn retag_ptr(&mut self, ptr: Pointer<M::Provenance>, ptr_type: lang::PtrType, fn_entry: bool) -> Result<Pointer<M::Provenance>> {
        self.memory.retag_ptr(ptr, ptr_type, fn_entry)
    }

    /// Checks that `size` is not too large for the Memory.
    pub fn valid_size(size: Size) -> bool {
        M::valid_size(size)
    }
}
```
