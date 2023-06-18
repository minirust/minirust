# MiniRust atomic memory

This is a wrapper for a memory that distinguishes between non-atomic and atomic memory accesses.
For now atomicity is ignored; this will change in the future.

```rust
pub struct AtomicMemory<M: Memory> {
    memory: M,

    /// The Id of the current thread
    current_thread: ThreadId,
    /// List of all memory access done by the active thread in the current step.
    current_accesses: List<Access>,

    /// The Id of the last thread
    last_thread: ThreadId,
    /// List of all memory accesses done by the last thread in the last step.
    last_accesses: List<Access>,
}

/// The different kinds of atomicity.
pub enum Atomicity {
    /// A sequentially consistent atomic access.
    Atomic,

    /// A non-atomic memory access.
    None,
}

/// Internal type used to track the type of a memory access.
enum AccessType {
    Store,
    Load,
}

/// Access contains all information the data race detection needs about a single access.
struct Access {
    ty: AccessType,
    atomicity: Atomicity,
    addr: Address,
    len: Int,
}
```

## Interface

The atomic memory presents a very similar interface to the `Memory`.
It differs in both store and load where we also take the `Atomicity` of an operation.

```rust
impl<M: Memory> AtomicMemory<M> {
    pub fn new() -> Self {
        Self { 
            memory: M::new(),
            current_thread: ThreadId::ZERO,
            current_accesses: list![],
            last_thread: ThreadId::ZERO,
            last_accesses: list![],
        }
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

    /// Write some bytes to memory and check for data races.
    pub fn store(&mut self, atomicity: Atomicity, ptr: Pointer<M::Provenance>, bytes: List<AbstractByte<M::Provenance>>, align: Align) -> Result {
        let access = Access::new(AccessType::Store, atomicity, ptr.addr, bytes.len());
        self.track_access(access)?;

        self.memory.store(ptr, bytes, align)
    }

    /// Read some bytes from memory and check for data races.
    pub fn load(&mut self, atomicity: Atomicity, ptr: Pointer<M::Provenance>, len: Size, align: Align) -> Result<List<AbstractByte<M::Provenance>>> {
        let access = Access::new(AccessType::Load, atomicity, ptr.addr, len.bytes());
        self.track_access(access)?;

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

## Data race detection

Here we define the operations needed to make data race detection.

```rust
impl<M: Memory> AtomicMemory<M> {
    /// Checks if this access is in a data race with any access that happend in the last access.
    /// It keeps track of the access for the next step.
    fn track_access(&mut self, access: Access) -> Result {
        self.current_accesses.push(access);

        if self.current_thread == self.last_thread { return Ok(()) }

        if self.last_accesses.any(|other| access.races(&other)) {
            throw_ub!("Data races");
        }

        Ok(())
    }

    /// Take meassures to track the next execution step.
    pub fn next_step(&mut self, thread: ThreadId) {
        self.last_thread = self.current_thread;
        self.last_accesses = self.current_accesses;

        self.current_thread = thread;
        self.current_accesses = list![];
    }
}

impl Access {
    /// Indicates if a races happend between the two given accesses.
    /// We assume they happen on different threads.
    fn races(&self, other: &Self) -> bool {
        // At least one access modifies the data.
        if self.ty == AccessType::Load && other.ty == AccessType::Load { return false; }

        // At least one access is non atomic
        if self.atomicity == Atomicity::Atomic && other.atomicity == Atomicity::Atomic { return false; }

        // The accesses overlap.
        let end_addr = self.addr + self.len;
        let other_end_addr = other.addr + other.len;
        end_addr > other.addr && other_end_addr > self.addr
    }
}
```

## Utility

```rust
impl Access {
    fn new(ty: AccessType, atomicity: Atomicity, addr: Address, len: Int) -> Self {
        Access {
            ty,
            atomicity,
            addr,
            len,
        }
    }
}
```
