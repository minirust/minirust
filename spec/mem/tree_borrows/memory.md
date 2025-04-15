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
```

The memory itself largely reuses the basic memory infrastructure, with the tree as extra state.

```rust
pub struct TreeBorrowsMemory<T: Target> {
    mem: BasicMemory<T, Path, TreeBorrowsAllocationExtra>,
}

pub struct TreeBorrowsFrameExtra {
    /// Our per-frame state is the list of nodes that are protected by this call.
    protectors: List<TreeBorrowsProvenance>,
}

impl TreeBorrowsFrameExtra {
    fn new() -> Self { Self { protectors: List::new() } }
}
```

Here we define some helper methods to implement the memory interface.

```rust
impl<T: Target> TreeBorrowsMemory<T> {
    /// Create a new node for a pointer (reborrow)
    fn reborrow(
        &mut self, 
        ptr: ThinPointer<TreeBorrowsProvenance>,
        pointee_size: Size,
        permission: Permission,
        protected: Protected,
        frame_extra: &mut TreeBorrowsFrameExtra,
    ) -> Result<ThinPointer<TreeBorrowsProvenance>> {
        // Make sure the pointer is dereferenceable.
        self.mem.check_ptr(ptr, pointee_size)?;
        // However, ignore the result of `check_ptr`: even if pointee_size is 0, we want to create a child pointer.
        let Some((alloc_id, parent_path)) = ptr.provenance else {
            assert!(pointee_size.is_zero());
            // Pointers without provenance cannot access any memory, so giving them a new
            // tag makes no sense.
            return ret(ptr);
        };

        let child_path = self.mem.allocations.mutate_at(alloc_id.0, |allocation| {
            // Create the new child node
            let child_node = Node {
                children: List::new(),
                location_states: LocationState::new_list(permission, allocation.size()),
                protected,
            };

            // Add the new node to the tree
            let child_path = allocation.extra.root.add_node(parent_path, child_node);

            // If this is a non-zero-sized reborrow, perform read on the new child, updating all nodes accordingly.
            if pointee_size.bytes() > 0 {
                let offset = Offset::from_bytes(ptr.addr - allocation.addr).unwrap();
                allocation.extra.root.access(Some(child_path), AccessKind::Read, offset, pointee_size)?;
            }

            ret::<Result<Path>>(child_path)
        })?;

        // Track the new protector
        if protected.yes() { frame_extra.protectors.push((alloc_id, child_path)); }

        // Create the child pointer and return it 
        ret(ThinPointer {
            provenance: Some((alloc_id, child_path)),
            ..ptr
        })
    }

    /// Remove the protector.
    /// `provenance` is the provenance of the protector.
    /// Perform a special implicit access on all locations that have been accessed.
    fn release_protector(&mut self, provenance: TreeBorrowsProvenance) -> Result {
        let (alloc_id, path) = provenance;
        self.mem.allocations.mutate_at(alloc_id.0, |allocation| {
            let protected_node = allocation.extra.root.get_node(path);

            if !allocation.live {
                match protected_node.protected {
                    Protected::Weak => return ret(()),
                    Protected::Strong =>
                        panic!("TreeBorrowsMemory::release_protector: strongly protected allocations can't be dead"),
                    Protected::No =>
                        panic!("TreeBorrowsMemory::release_protector: no protector"),
                }
            }

            allocation.extra.root.release_protector(Some(path), &protected_node.location_states)
        })
    }

    /// Compute the reborrow settings for the given pointer type.
    /// `None` indicates that no reborrow should happen.
    fn ptr_permissions(ptr_type: PtrType, fn_entry: bool) -> Option<(Permission, LayoutStrategy, Protected)> {
        match ptr_type {
            PtrType::Ref { mutbl, pointee } if !pointee.freeze && mutbl == Mutability::Immutable => {
                // Shared reference to interior mutable type: retagging is a NOP.
                None
            },
            PtrType::Ref { mutbl, pointee } if !pointee.unpin && mutbl == Mutability::Mutable => {
                // Mutable reference to pinning type: retagging is a NOP.
                None
            },
            PtrType::Ref { mutbl, pointee } => {
                let protected = if fn_entry { Protected::Strong } else { Protected::No };
                let permission = Permission::default(mutbl, pointee, protected);
                Some((permission, pointee.layout, protected))
            },
            PtrType::Box { pointee } => {
                let protected = if fn_entry { Protected::Weak } else { Protected::No };
                let permission = Permission::default(Mutability::Mutable, pointee, protected);
                Some((permission, pointee.layout, protected))
            },
            _ => None,
        }
    }
}
```

# Memory Operations

Then we implement the memory model interface for Tree Borrows.

```rust
impl<T: Target> Memory for TreeBorrowsMemory<T> {
    type Provenance = TreeBorrowsProvenance;
    type FrameExtra = TreeBorrowsFrameExtra;
    type T = T;

    fn new() -> Self {
        Self { mem: BasicMemory::new() }
    }

    fn allocate(&mut self, kind: AllocationKind, size: Size, align: Align) -> NdResult<ThinPointer<Self::Provenance>>  {
        // Create the root node for the tree.
        // Initially, we set the permission as `Unique`.
        let root = Node {
            children: List::new(),
            location_states: LocationState::new_list(Permission::Unique, size),
            protected: Protected::No,
        };
        let path = Path::new();
        let extra = TreeBorrowsAllocationExtra { root };
        self.mem.allocate(kind, size, align, path, extra)
    }

    fn deallocate(&mut self, ptr: ThinPointer<Self::Provenance>, kind: AllocationKind, size: Size, align: Align) -> Result {
        self.mem.deallocate(ptr, kind, size, align, |extra, path| {
            // Check that ptr has the permission to write the entire allocation.
            extra.root.access(Some(path), AccessKind::Write, Offset::ZERO, size)?;

            // Check that allocation is not strongly protected.
            // TODO: This makes it UB to deallocate memory even if the strong protector covers 0 bytes!
            // That's different from SB, and we might want to change it in the future.
            if extra.root.contains_strong_protector() {
                throw_ub!("Tree Borrows: deallocating strongly protected allocation")
            }

            ret(())
        })
    }

    fn load(&mut self, ptr: ThinPointer<Self::Provenance>, len: Size, align: Align) -> Result<List<AbstractByte<Self::Provenance>>> {
        self.mem.load(ptr, len, align, |extra, path, offset| {
            // Check for aliasing violations.
            extra.root.access(Some(path), AccessKind::Read, offset, len)
        })
    }

    fn store(&mut self, ptr: ThinPointer<Self::Provenance>, bytes: List<AbstractByte<Self::Provenance>>, align: Align) -> Result {
        let size = Size::from_bytes(bytes.len()).unwrap();
        self.mem.store(ptr, bytes, align, |extra, path, offset| {
            // Check for aliasing violations.
            extra.root.access(Some(path), AccessKind::Write, offset, size)
        })
    }

    fn dereferenceable(&self, ptr: ThinPointer<Self::Provenance>, len: Size) -> Result {
        self.mem.check_ptr(ptr, len)?;
        ret(())
    }

    fn retag_ptr(
        &mut self,
        frame_extra: &mut Self::FrameExtra,
        ptr: Pointer<Self::Provenance>,
        ptr_type: PtrType,
        fn_entry: bool,
        size_computer: impl Fn(LayoutStrategy, Option<PointerMeta<Self::Provenance>>) -> Size,
    ) -> Result<Pointer<Self::Provenance>> {
        ret(if let Some((permission, layout, protected)) = Self::ptr_permissions(ptr_type, fn_entry) {
            let pointee_size = size_computer(layout, ptr.metadata);
            self.reborrow(ptr.thin_pointer, pointee_size, permission, protected, frame_extra)?.widen(ptr.metadata)
        } else {
            ptr
        })
    }

    fn new_call() -> Self::FrameExtra {  Self::FrameExtra::new() }

    fn end_call(&mut self, extra: Self::FrameExtra) -> Result {
        extra.protectors.try_map(|provenance| self.release_protector(provenance))?;
        ret(())
    }

    fn leak_check(&self) -> Result {
        self.mem.leak_check()
    }
}
```
