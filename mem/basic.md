# MiniRust basic memory model

This is almost the simplest possible fully-feature implementation of the MiniRust memory model interface.
It does *not* model any kind of aliasing restriction, but otherwise should be enough to explain all the behavior and Undefined Behavior we see in Rust, in particular with respect to bounds-checks for memory accesses and pointer arithmetic.
This demonstrates well how the memory interface works, as well as the basics of "per-allocation provenance".
The full MiniRust memory model will likely be this basic model plus some [extra restrictions][Stacked Borrows] to ensure the program follows the aliasing rules; possibly with some extra tricks to [explain OOM-reducing optimizations](https://github.com/rust-lang/unsafe-code-guidelines/issues/328).

[Stacked Borrows]: https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/stacked-borrows.md

## Data structures

The provenance tracked by this memory model is just an ID that identifies which allocation the pointer points to.
(We will pretend we can split the `impl ... for` block into multiple smaller blocks.)

```rust
pub struct AllocId(Int);

impl Memory for BasicMemory {
    type Provenance = AllocId;
}
```

The data tracked by the memory is fairly simple: for each allocation, we track its data contents, its absolute integer address in memory, the alignment it was created with (the size is implicit in the length of the contents), and whether it is still alive (or has already been deallocated).

```rust
struct Allocation {
    /// The data stored in this allocation.
    data: List<AbstractByte<AllocId>>,
    /// The address where this allocation starts.
    /// This is never 0, and `addr + data.len()` fits into a `usize`.
    addr: Int,
    /// The alignment that was requested for this allocation.
    /// `addr` will be a multiple of this.
    align: Align,
    /// Whether this allocation is still live.
    live: bool,
}
```

Memory then consists of a map tracking the allocation for each ID, stored as a list (since we assign IDs consecutively).

```rust
pub struct BasicMemory {
    allocations: List<Allocation>,
}
```

The model represents a 64-bit little-endian machine.

```rust
impl Memory for BasicMemory {
    const PTR_SIZE: Size = Size::from_bits_const(64);
    const PTR_ALIGN: Align = Align::from_bits_const(64).unwrap();
    const ENDIANNESS: Endianness = LittleEndian;
}
```

```rust
impl Memory for BasicMemory {
    fn new() -> Self {
        Self { allocations: List::new() }
    }
}
```

## Operations

We start with some helper operations.

```rust
impl Allocation {
    fn size(self) -> Size { Size::from_bytes(self.data.len()) }

    fn overlaps(self, other_addr: Int, other_size: Size) -> bool {
        let end_addr = self.addr + self.size().bytes();
        let other_end_addr = other_addr + other_size.bytes();
        if end_addr <= other_addr || other_end_addr <= self.addr {
            // Our end is before their beginning, or vice versa -- we do not overlap.
            false
        } else {
            true
        }
    }
}
```

Then we implement creating and removing allocations.

```rust
impl Memory for BasicMemory {
    fn allocate(&mut self, size: Size, align: Align) -> NdResult<Pointer<AllocId>> {
        // Reject too large allocations. Size must fit in `isize`.
        if !Self::valid_size(size) {
            throw_ub!("asking for a too large allocation");
        }
        // Pick a base address. We use daemonic non-deterministic choice,
        // meaning the program has to cope with every possible choice.
        // FIXME: This makes OOM (when there is no possible choice) into "no behavior",
        // which is not what we want.
        let distr = specr::IntDistribution {
            start: Int::ONE,
            end: Int::from(2).pow(Self::PTR_SIZE.bits()),
            divisor: align.bytes(),
        };
        let addr = pick(distr, |addr: Int| {
            // Pick a strictly positive integer...
            if addr <= 0 { return false; }
            // ... that is suitably aligned...
            if addr % align.bytes() != 0 { return false; }
            // ... such that addr+size is in-bounds of a `usize`...
            if !(addr+size.bytes()).in_bounds(Unsigned, Self::PTR_SIZE) { return false; }
            // ... and it does not overlap with any existing live allocation.
            if self.allocations.any(|a| a.live && a.overlaps(addr, size)) { return false; }
            // If all tests pass, we are good!
            true
        })?;

        // Compute allocation.
        let allocation = Allocation {
            addr,
            align,
            live: true,
            data: list![AbstractByte::Uninit; size.bytes()],
        };

        // Insert it into list, and remember where.
        let id = AllocId(self.allocations.len());
        self.allocations.push(allocation);

        // And we are done!
        ret(Pointer { addr, provenance: Some(id) })
    }

    fn deallocate(&mut self, ptr: Pointer<AllocId>, size: Size, align: Align) -> Result {
        let Some(id) = ptr.provenance else {
            throw_ub!("deallocating invalid pointer")
        };
        // This lookup will definitely work, since AllocId cannot be faked.
        let allocation = self.allocations[id.0];

        // Check a bunch of things.
        if !allocation.live {
            throw_ub!("double-free");
        }
        if ptr.addr != allocation.addr {
            throw_ub!("deallocating with pointer not to the beginning of its allocation");
        }
        if size != allocation.size() {
            throw_ub!("deallocating with incorrect size information");
        }
        if align != allocation.align {
            throw_ub!("deallocating with incorrect alignment information");
        }

        // Mark it as dead. That's it.
        self.allocations.mutate_at(id.0, |allocation| {
            allocation.live = false;
        });

        ret(())
    }
}
```

The key operations of a memory model are of course handling loads and stores.
The helper function `check_ptr` we define for them is also used to implement the final part of the memory API, `dereferenceable`.

```rust
impl BasicMemory {
    /// Check if the given pointer is dereferenceable for an access of the given
    /// length and alignment. For dereferenceable, return the allocation ID and
    /// offset; this can be missing for invalid pointers and accesses of size 0.
    fn check_ptr(&self, ptr: Pointer<AllocId>, len: Size, align: Align) -> Result<Option<(AllocId, Size)>> {
        // Basic address sanity checks.
        if ptr.addr == 0 {
            throw_ub!("dereferencing null pointer");
        }
        if ptr.addr % align.bytes() != 0 {
            throw_ub!("pointer is insufficiently aligned");
        }
        // For zero-sized accesses, this is enough.
        // (Provenance monotonicity says that if we allow zero-sized accesses
        // for `None` provenance we have to allow it for all provenance.)
        // FIXME: This is *not* what rustc does, since from this it follows that
        // `offset(0)` is allowed on all non-null pointers which does not match
        // the LLVM IR rustc generates.
        if len.is_zero() {
            return ret(None);
        }
        // Now try to access the allocation information.
        let Some(id) = ptr.provenance else {
            // An invalid pointer.
            throw_ub!("non-zero-sized access with invalid pointer")
        };
        let allocation = self.allocations[id.0];

        if !allocation.live {
            throw_ub!("memory accessed after deallocation");
        }

        // Compute relative offset, and ensure we are in-bounds.
        let offset_in_alloc = ptr.addr - allocation.addr;
        if offset_in_alloc < 0 || offset_in_alloc + len.bytes() > allocation.size().bytes() {
            throw_ub!("out-of-bounds memory access");
        }
        // All is good!
        ret(Some((id, Size::from_bytes(offset_in_alloc))))
    }
}

impl Memory for BasicMemory {
    fn load(&mut self, ptr: Pointer<AllocId>, len: Size, align: Align) -> Result<List<AbstractByte<AllocId>>> {
        let Some((id, offset)) = self.check_ptr(ptr, len, align)? else {
            return ret(list![]);
        };
        let allocation = &self.allocations[id.0];

        // Slice into the contents, and copy them to a new list.
        ret(allocation.data.subslice_with_length(offset.bytes(), len.bytes()))
    }

    fn store(&mut self, ptr: Pointer<Self::Provenance>, bytes: List<AbstractByte<Self::Provenance>>, align: Align) -> Result {
        let size = Size::from_bytes(bytes.len());
        let Some((id, offset)) = self.check_ptr(ptr, size, align)? else {
            return ret(());
        };

        // Slice into the contents, and put the new bytes there.
        self.allocations.mutate_at(id.0, |allocation| {
            allocation.data.write_subslice_at_index(offset.bytes(), bytes);
        });

        ret(())
    }

    fn dereferenceable(&self, ptr: Pointer<Self::Provenance>, size: Size, align: Align) -> Result {
        self.check_ptr(ptr, size, align)?;

        ret(())
    }
}
```

We don't have aliasing requirements in this model, so we only check dereferencability for retagging.

```rust
impl Memory for BasicMemory {
    fn retag_ptr(&mut self, ptr: Pointer<Self::Provenance>, ptr_type: lang::PtrType, _fn_entry: bool) -> Result<Pointer<Self::Provenance>> {
        let layout = match ptr_type {
            lang::PtrType::Ref { pointee, .. } => pointee,
            lang::PtrType::Box { pointee } => pointee,
            lang::PtrType::Raw { pointee } => pointee,
        };
        self.check_ptr(ptr, layout.size, layout.align)?;

        ret(ptr)
    }
}
```

A size is valid, whenever it is non-negative and in-bounds for signed `PTR_SIZE`.

```rust
impl Memory for BasicMemory {
    fn valid_size(size: Size) -> bool {
        size.bytes().in_bounds(Signed, Self::PTR_SIZE)
    }
}
```
