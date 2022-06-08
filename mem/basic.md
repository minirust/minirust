# MiniRust basic memory model

This is almost the simplest possible fully-feature implementation of the MiniRust memory model interface.
It does *not* model any kind of aliasing restriction, but otherwise should be enough to explain all the behavior and Undefined Behavior we see in Rust, in particular with respect to bounds-checks for memory accesses and pointer arithmetic.
This demonstrates well how the memory interface works, as well as the basics of "per-allocation provenance".
The full MiniRust memory model will likely be this basic model plus some [extra restrictions][Stacked Borrows] to ensure the program follows the aliasing rules; possibly with some extra tricks to [explain OOM-reducing optimizations](https://github.com/rust-lang/unsafe-code-guidelines/issues/328).

[Stacked Borrows]: https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/stacked-borrows.md

## Data structures

The provenance tracked by this memory model is just an ID that identifies which allocation the pointer points to.

```rust
#[derive(PartialEq, Eq)]
struct AllocId(BigInt);
```

The data tracked by the memory is fairly simple: for each allocation, we track its contents, its absolute integer address in memory, the alignment it was created with (the size is implicit in the length of the contents), and whether it is still alive (or has already been deallocated).

```rust
struct Allocation {
    /// The data stored in this allocation.
    contents: List<AbstractByte<Provenance>>,
    /// The address where this allocation starts.
    /// This is never 0, and `addr + contents.len()` fits into a `usize`.
    addr: BigInt,
    /// The alignment that was requested for this allocation.
    /// `addr` will be a multiple of this.
    align: Align,
    /// Whether this allocation is still live.
    live: bool,
}
```

Memory then consists of a map tracking the allocation for each ID, stored as a list (since we assign IDs consecutively).

```rust
struct BasicMemory {
    allocations: List<Allocation>,
}
```

## Operations

We start with some helper operations.

```rust
impl Allocation {
    fn size(self) -> BigInt { self.contents.len() }

    fn overlaps(self, other_addr: BigInt, other_size: Size) -> bool {
        let end_addr = self.addr + self.size();
        let other_end_addr = other_addr + other_size;
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
(We will just pretend we can split the `impl ... for` block into multiple smaller blocks.)

```rust
impl MemoryInterface for BasicMemory {
    type Provenance = AllocId;

    fn allocate(&mut self, size: Size, align: Align) -> Result<Pointer<AllocId>> {
        // Pick a base address. We use daemonic non-deterministic choice,
        // meaning the program has to cope with every possible choice.
        // FIXME: This makes OOM (when there is no possible choice) into "no behavior",
        // which is not what we want.
        let addr = pick(|addr| {
            // Pick a strictly positive integer...
            if addr <= 0 { return false; }
            // ... such that addr+size is in-bounds of a `usize`...
            if !(addr+size).in_bounds(Unsigned, PTR_SIZE) { return false; }
            // ... and it does not overlap with any existing live allocation.
            if self.allocations.values().any(|a| a.live && a.overlaps(addr, size)) { return false; }
            // If all tests pass, we are good!
            true
        })?;

        // Compute allocation.
        let allocation = Allocation {
            addr,
            align,
            live: true,
            contents: list![AbstractByte::Uninit; size],
        }

        // Insert it into list, and remember where.
        let id = AllocId(self.allocations.len());
        self.allocations.push(allocation);
        // And we are done!
        Pointer { addr, provenance: Some(id) }
    }

    fn deallocate(&mut self, ptr: Pointer<AllocId>, size: Size, align: Align) -> Result {
        let Some(id) = ptr.provenance else {
            throw_ub!("deallocating invalid pointer")
        };
        // This lookup will definitely work, since AllocId cannot be faked.
        let allocation = &mut self.allocations[id.0];
        if !allocation.live {
            throw_ub!("double-free");
        }
        if ptr.addr != allocation.addr {
            throw_ub!("deallocating with pointer not to the beginning of its allocation");
        }

        // Mark it as dead. That's it.
        allocation.live = false.
    }
}
```
