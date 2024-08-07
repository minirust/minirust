# MiniRust Tree Borrows

For background on Tree Borrows, see:

1. [Neven's posts on Tree Borrows](https://perso.crans.org/vanille/treebor)
2. [From Stacks to Trees: A new aliasing model for Rust](https://www.ralfj.de/blog/2023/06/02/tree-borrows.html)

Similar to the [Basic Memory Model](../basic.md), we need to first define some basic data structures:
the core date structure managing the tree is defined in [tree.md](tree.md), and the core state machine can be found in [state_machine.md](state_machine.md).

The model then tracks a tree for each allocation:
```rust
struct TreeBorrowsAllocationExtra {
    root: Node,
}
```

We use a *path* to identify each node and track its location in the tree. A path is represented as a list of indices $[i_1, i_2, ..., i_k]$, where each index indicates which branch to take next.
Below is an illustrated example:
```
Consider the following tree
      A
     / \
    B   C
   / \   \
  D  E    F
The path from A to A is represented as [].
The path from A to B is represented as [0]
The path from A to C is represented as [1].
The path from A to D is represented as [0, 0].
The path from A to E is represented as [0, 1].
The path from A to F is represented as [1, 1].
```

```rust
/// The index of a child in the list of child nodes.
type ChildId = Int;
/// A path from the root of a tree to some node inside the tree.
type Path = List<ChildId>;
```

Then we can define the provenance of Tree Borrows as a pair consisting of the path and the allocation ID.

```rust
type TreeBorrowsProvenance = (AllocId, Path);
type TreeBorrowsAllocation = Allocation<TreeBorrowsProvenance, TreeBorrowsAllocationExtra>;
```


```rust
pub struct TreeBorrowsMemory<T: Target> {
    allocations: List<TreeBorrowsAllocation>,
    // FIXME: specr should add this automatically
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Target> Memory for TreeBorrowsMemory<T> {
    type Provenance = TreeBorrowsProvenance;
    type T = T;

    fn new() -> Self {
        Self { allocations: List::new(), _phantom: std::marker::PhantomData }
    }
}
```

Here we define some helper methods to implement the memory interface.

```rust
impl<T: Target> TreeBorrowsMemory<T> {
    /// create a location state list for an allocation:
    /// all locations inside the permission have the given permission.
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
        let Some((alloc_id, parent_path, offset)) = self.check_ptr(ptr, pointee_size)? else {
            return ret(ptr);
        };

        let child_path = self.allocations.mutate_at(alloc_id.0, |allocation| {
            // Create the new child node
            let child_node = Node {
                children: List::new(),
                location_states: Self::init_location_states(permission, allocation.size()),
            };

            // Add the new node to the tree
            let child_path = allocation.extra.root.add_node(parent_path, child_node)?;

            // Perform read on the new child, updating all nodes accordingly.
            allocation.extra.root.access(Some(child_path), AccessKind::Read, offset, pointee_size)?;

            ret::<Result<Path>>(child_path)
        })?;

        // Create the child pointer and return it 
        ret(Pointer {
            provenance: Some((alloc_id, child_path)),
            ..ptr
        })
    }


    /// Return the borrow tag, allocation ID and offset
    fn check_ptr(&self, ptr: Pointer<TreeBorrowsProvenance>, len: Size) -> Result<Option<(AllocId, Path, Size)>> {
        // For zero-sized accesses, there is nothing to check.
        // (Provenance monotonicity says that if we allow zero-sized accesses
        // for `None` provenance we have to allow it for all provenance.)
        if len.is_zero() {
            return ret(None);
        }
        // We do not even have to check for null, since no allocation will ever contain that address.
        // Now try to access the allocation information.
        let Some((alloc_id, path)) = ptr.provenance else {
            // An invalid pointer.
            throw_ub!("dereferencing pointer without provenance");
        };
        let allocation = self.allocations[alloc_id.0];

        // Compute relative offset
        let offset = allocation.offset_in_alloc(ptr.addr, len)?;

        // All is good!
        ret(Some((alloc_id, path, offset)))
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
        let addr = Allocation::pick_base_address::<T>(self.allocations, size, align)?;

        // Calculate the proverance for the root node.
        let alloc_id = AllocId(self.allocations.len());

        // Create the root node for the tree.
        // Initially, we set the permission as `Active`.
        let root = Node {
            children: List::new(),
            location_states: Self::init_location_states(Permission::Active, size),
        };

        // Path to the root node
        let path = List::new();

        let allocation = Allocation {
            addr,
            align,
            kind,
            live: true,
            data: list![AbstractByte::Uninit; size.bytes()],
            extra: TreeBorrowsAllocationExtra { root },
        };

        self.allocations.push(allocation);

        ret(Pointer { addr, provenance: Some((alloc_id, path)) })
    }
}

impl<T: Target> Memory for TreeBorrowsMemory<T> {
    fn deallocate(&mut self, ptr: Pointer<Self::Provenance>, kind: AllocationKind, size: Size, align: Align) -> Result {
        let Some((alloc_id, path)) = ptr.provenance else {
            throw_ub!("deallocating invalid pointer")
        };
        // This lookup will definitely work, since AllocId cannot be faked.
        let mut allocation = self.allocations[alloc_id.0];

        allocation.deallocation_check(ptr.addr, kind, size, align)?;

        // Check that ptr has the permission to write the entire allocation.
        allocation.extra.root.access(Some(path), AccessKind::Write, Size::ZERO, size)?;

        // Mark it as dead. That's it.
        self.allocations.mutate_at(alloc_id.0, |allocation| {
            allocation.live = false;
        });

        ret(())
    }
}
```

### Load Operation

```rust
impl<T: Target> Memory for TreeBorrowsMemory<T> {
    fn load(&mut self, ptr: Pointer<Self::Provenance>, len: Size, align: Align) -> Result<List<AbstractByte<Self::Provenance>>> {
       let Some((alloc_id, path, offset)) = self.check_ptr(ptr, len)? else {
            return ret(list![]);
        };

        let data = self.allocations.mutate_at(alloc_id.0, |allocation| {
            // Check for aliasing violations.
            allocation.extra.root.access(Some(path), AccessKind::Read, offset, len)?;

            // Load the data.
            allocation.load(ptr.addr, offset, len, align)
        })?;

        ret(data)
    }
}
```

### Store Operation

```rust
impl<T: Target> Memory for TreeBorrowsMemory<T> {
    fn store(&mut self, ptr: Pointer<Self::Provenance>, bytes: List<AbstractByte<Self::Provenance>>, align: Align) -> Result {
        let size = Size::from_bytes(bytes.len()).unwrap();
        let Some((alloc_id, path, offset)) = self.check_ptr(ptr, size)? else {
            return ret(());
        };

        self.allocations.mutate_at(alloc_id.0, |allocation| {
            // Check for aliasing violations.
            allocation.extra.root.access(Some(path), AccessKind::Write, offset, size)?;

            // Store the data.
            allocation.store(ptr.addr, offset, bytes, align)
        })?;

        ret(())
    }
}
```

### Retagging Operation

```rust
impl<T: Target> Memory for TreeBorrowsMemory<T> {
    fn retag_ptr(&mut self, ptr: Pointer<Self::Provenance>, ptr_type: PtrType, _fn_entry: bool) -> Result<Pointer<Self::Provenance>> {
        match ptr_type {
            PtrType::Ref { mutbl, pointee } if !pointee.freeze && mutbl == Mutability::Immutable => ret(ptr),
            PtrType::Ref { mutbl, pointee } => self.reborrow(ptr, pointee.size, Permission::default(mutbl, pointee.freeze)),
            PtrType::Box { pointee } => self.reborrow(ptr, pointee.size, Permission::default(Mutability::Mutable, pointee.freeze)),
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
        Allocation::leak_check(self.allocations)
    }
}
```
