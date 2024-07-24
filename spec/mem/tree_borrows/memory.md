# MiniRust Tree Borrows

To check the details on Tree Borrows

1. [From Stacks to Trees: A new aliasing model for Rust](https://www.ralfj.de/blog/2023/06/02/tree-borrows.html)
1. [Neven's posts on Tree Borrows](https://perso.crans.org/vanille/treebor)

We define the core date structure *Tree* in the [tree.md](tree.md) and the core state machine in the [state_machine.md](state_machine.md).

Similar to the [Basic Memory Model](../basic.md), we need to first define some basic data structures.

Unlike `BasicMemory`, `TreeBorrowsMemory` also tracks an ID for each pointer in the provenance, called a *Borrow Tag*.

```rust
pub struct BorTag(Int);
pub type TreeBorrowsProvenance = (BorTag, AllocId);
```

```rust
pub struct TreeBorrowsMemory<T: Target> {
    tree_allocs: List<TreeBorrowsAllocation>,
    /// Next unused borrow tag.
    next_tag: BorTag,
    // FIXME: specr should add this automatically
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Target> Memory for TreeBorrowsMemory<T> {
    type Provenance = TreeBorrowsProvenance;
    type T = T;

    fn new() -> Self {
        Self { tree_allocs: List::new(), next_tag: BorTag(Int::ZERO), _phantom: std::marker::PhantomData }
    }
}
```

```rust
pub struct TreeBorrowsAllocation {
    /// The same allocation data as the basic memory model.
    allocation: Allocation<TreeBorrowsProvenance>,
    /// The **TREE** for the Tree Borrows.
    tree: Tree,
}
```

Here we define some helper methods to implement the memory interface.

```rust
impl<T: Target> TreeBorrowsMemory<T> {
    /// Given the permission and the allocation size,
    /// create an initialized location state list for an allocation.
    fn init_location_states(permission: Permission, alloc_size: Size) -> List<LocationState> {
        let mut location_states = List::new();
        for _ in Int::ZERO..alloc_size.bytes() {
            location_states.push(LocationState {
                accessed: Accessed::No,
                permission,
            });
        }

        location_states
    }

    /// Create a new node for a pointer (reborrow)
    fn reborrow(
        &mut self, 
        ptr: Pointer<TreeBorrowsProvenance>,
        pointee_size: Size,
        permission: Permission
    ) -> Result<Pointer<TreeBorrowsProvenance>> {
        let Some((parent_tag, alloc_id, offset)) = self.check_ptr(ptr, pointee_size)? else {
            return ret(ptr);
        };

        let mut tree_alloc = self.tree_allocs[alloc_id.0];
        let allocation = tree_alloc.allocation;

        // Create the new child node
        let child_node = Node {
            parent: Some(parent_tag),
            children: List::new(),
            location_states: Self::init_location_states(permission, allocation.size()),
        };

        let child_tag = self.next_tag();

        // Add the new node to the parent's children list
        tree_alloc.tree.add_child(parent_tag, child_tag, child_node)?;

        // Perform child read to all nodes
        tree_alloc.tree.access(child_tag, AccessKind::Read, offset, pointee_size)?;
        self.tree_allocs.set(alloc_id.0, tree_alloc);

        // Create the child pointer and return it 
        ret(Pointer {
            provenance: Some((child_tag, alloc_id)),
            ..ptr
        })
    }

    fn next_tag(&mut self) -> BorTag {
        let tag = self.next_tag;
        self.next_tag = BorTag(self.next_tag.0 + Int::ONE);
        tag
    }
}
```

# Memory Operations
Then we implement the memory model interface for the Tree Borrow.

### Allocate and Deallocate

We create a new tree for one allocation

```rust
impl<T: Target> Memory for TreeBorrowsMemory<T> {
    fn allocate(&mut self, kind: AllocationKind, size: Size, align: Align) -> NdResult<Pointer<Self::Provenance>>  {
        // Reject too large allocations. Size must fit in `isize`.
        if !T::valid_size(size) {
            throw_ub!("asking for a too large allocation");
        }
        // Pick a base address. We use daemonic non-deterministic choice,
        // meaning the program has to cope with every possible choice.
        // FIXME: This makes OOM (when there is no possible choice) into "no behavior",
        // which is not what we want.
        let distr = libspecr::IntDistribution {
            start: Int::ONE,
            end: Int::from(2).pow(Self::T::PTR_SIZE.bits()),
            divisor: align.bytes(),
        };
        let addr = pick(distr, |addr: Address| {
            // Pick a strictly positive integer...
            if addr <= 0 { return false; }
            // ... that is suitably aligned...
            if !align.is_aligned(addr) { return false; }
            // ... such that addr+size is in-bounds of a `usize`...
            if !(addr+size.bytes()).in_bounds(Unsigned, Self::T::PTR_SIZE) { return false; }
            // ... and it does not overlap with any existing live allocation.
            if self.tree_allocs.any(|ta| ta.allocation.live && ta.allocation.overlaps(addr, size)) { return false; }
            // If all tests pass, we are good!
            true
        })?;

        // Calculate the proverance for the root node
        let bor_tag = self.next_tag();
        let alloc_id = AllocId(self.tree_allocs.len());

        // Create the root node for the tree.
        // Intially, we set the permission as `Active`
        let root_node = Node { 
            parent: None,
            children: List::new(),
            location_states: Self::init_location_states(Permission::Active, size),
        };

        let mut nodes = Map::new();
        nodes.insert(bor_tag, root_node);
        
        // Create the tree
        let tree = Tree {
            root_tag: bor_tag, 
            nodes,
        };
        
        let allocation = Allocation {
            addr,
            align,
            kind,
            live: true,
            data: list![AbstractByte::Uninit; size.bytes()],
        };

        let tree_alloc = TreeBorrowsAllocation {
            allocation,
            tree,
        };

        self.tree_allocs.push(tree_alloc);

        ret(Pointer { addr, provenance: Some((bor_tag, alloc_id)) })
    }
}

impl<T: Target> Memory for TreeBorrowsMemory<T> {
    fn deallocate(&mut self, ptr: Pointer<Self::Provenance>, kind: AllocationKind, size: Size, align: Align) -> Result {
        let Some((bor_tag, alloc_id)) = ptr.provenance else {
            throw_ub!("deallocating invalid pointer")
        };
        // This lookup will definitely work, since AllocId cannot be faked.
        let mut tree_alloc = self.tree_allocs[alloc_id.0];

        tree_alloc.allocation.deallocation_check(ptr.addr, kind, size, align)?;

        // check that ptr has the permission to write the entire allocation
        tree_alloc.tree.access(bor_tag, AccessKind::Write, Size::ZERO, size)?;

        // Mark it as dead. That's it.
        self.tree_allocs.mutate_at(alloc_id.0, |tree_alloc| {
            tree_alloc.allocation.live = false;
        });

        ret(())
    }
}
```

### Load Operation

Corresponding to the `AccessKind::Read`.

```rust
impl<T: Target> Memory for TreeBorrowsMemory<T> {
    fn load(&mut self, ptr: Pointer<Self::Provenance>, len: Size, align: Align) -> Result<List<AbstractByte<Self::Provenance>>> {
        if !align.is_aligned(ptr.addr) {
            throw_ub!("Tree Borrows: load from a misaligned pointer");
        }

       let Some((bor_tag, alloc_id, offset)) = self.check_ptr(ptr, len)? else {
            return ret(list![]);
        };

        // Recursively update the tree and check the existence of UBs 
        let mut tree_alloc = self.tree_allocs[alloc_id.0];

        // Read the data
        let data = tree_alloc.allocation.read(ptr.addr, offset, len, align)?;

        tree_alloc.tree.access(bor_tag, AccessKind::Read, offset, len)?;

        self.tree_allocs.set(alloc_id.0, tree_alloc);

        ret(data)
    }
}
```

### Store Operation

Corresponding to the `AccessKind::Write`

```rust
impl<T: Target> Memory for TreeBorrowsMemory<T> {
    fn store(&mut self, ptr: Pointer<Self::Provenance>, bytes: List<AbstractByte<Self::Provenance>>, align: Align) -> Result {
        if !align.is_aligned(ptr.addr) {
            throw_ub!("Tree Borrows: store to a misaligned pointer");
        }

        let size = Size::from_bytes(bytes.len()).unwrap();
        let Some((bor_tag, alloc_id, offset)) = self.check_ptr(ptr, size)? else {
            return ret(());
        };

        let mut tree_alloc = self.tree_allocs[alloc_id.0];

        // Store the data
        tree_alloc.allocation.write(ptr.addr, offset, bytes, align)?;

        // State Transition
        tree_alloc.tree.access(bor_tag, AccessKind::Write, offset, size)?;

        self.tree_allocs.set(alloc_id.0, tree_alloc);

        ret(())
    }
}
```

### Retagging Operation

```rust
impl<T: Target> Memory for TreeBorrowsMemory<T> {
    fn retag_ptr(&mut self, ptr: Pointer<Self::Provenance>, ptr_type: PtrType, _fn_entry: bool) -> Result<Pointer<Self::Provenance>> {
        match ptr_type {
            PtrType::Ref { mutbl, pointee } => {
                let permission = match mutbl {
                    Mutability::Mutable => Permission::Reserved,
                    Mutability::Immutable => Permission::Frozen,
                };
                self.reborrow(ptr, pointee.size, permission)
            },
            PtrType::Box { pointee } => self.reborrow(ptr, pointee.size, Permission::Reserved),
            _ => ret(ptr),
        }
    }
}
```

### Checking Operation

```rust
impl<T: Target> Memory for TreeBorrowsMemory<T> {
    fn dereferenceable(&self, ptr: Pointer<Self::Provenance>, len: Size) -> Result {
        self.check_ptr(ptr, len)?;
        ret(())
    }
}
```

```rust
impl<T: Target> Memory for TreeBorrowsMemory<T> {
    fn leak_check(&self) -> Result {
        for tree_alloc in self.tree_allocs {
            if tree_alloc.allocation.live {
                match tree_alloc.allocation.kind {
                    // These should all be gone.
                    AllocationKind::Heap => throw_memory_leak!(),
                    // These we can still have at the end.
                    AllocationKind::Global | AllocationKind::Function | AllocationKind::Stack => {}
                }
            }
        }
        ret(())
    }
}
```

```rust
impl<T: Target> TreeBorrowsMemory<T> {
    /// Return the borrow tag, allocation ID and offset
    fn check_ptr(&self, ptr: Pointer<TreeBorrowsProvenance>, len: Size) -> Result<Option<(BorTag, AllocId, Size)>> {
        // For zero-sized accesses, there is nothing to check.
        // (Provenance monotonicity says that if we allow zero-sized accesses
        // for `None` provenance we have to allow it for all provenance.)
        if len.is_zero() {
            return ret(None);
        }
        // We do not even have to check for null, since no allocation will ever contain that address.
        // Now try to access the allocation information.
        let Some((bor_tag, alloc_id)) = ptr.provenance else {
            // An invalid pointer.
            throw_ub!("dereferencing pointer without provenance");
        };
        let allocation = self.tree_allocs[alloc_id.0].allocation;

        // Compute relative offset
        let offset = allocation.offset_in_alloc(ptr.addr, len)?;

        // All is good!
        ret(Some((bor_tag, alloc_id, offset)))
    }
}
```
