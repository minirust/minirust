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
fn act_on_bytes(nonfreeze_bytes: List<(Offset, Offset)>, start: Offset, size: Size, mut f: impl FnMut(Offset, bool) -> Result) -> Result {
    assert!(nonfreeze_bytes.iter().is_sorted_by(|a, b| a.0 <= b.0));

    let padded_front = std::iter::once((Size::ZERO, Size::ZERO)).chain(nonfreeze_bytes.iter());
    let padded_back = nonfreeze_bytes.iter().chain(std::iter::once((size, size)));

    // The following `zip` produces iterators that look like this:
    //
    // current: (0, 0)       first pair     second pair       …        last pair
    // next:    first pair   second pair    …             last pair    (size, size)
    //
    // This is done so that we know when the "next" range of UnsafeCells starts.
    // In the first iteration, we "see" an UnsafeCell between offsets 0 and 0,
    // and we know that the bytes from offset 0 until `(first pair).0` are free
    // of UnsafeCells.  Only in the second iteration do we actually see the
    // first real UnsafeCell.  In general, the loop has n+1 iterations, since we
    // visit the area before the first and after the last UnsafeCell.
    for (current, next) in padded_front.zip(padded_back) {
        // These bytes are inside an UnsafeCell
        for offset in current.0.bytes()..current.1.bytes() {
            f(Offset::from_bytes(offset).unwrap() + start, false)?
        }
        // These bytes are not in an UnsafeCell
        for offset in current.1.bytes()..next.0.bytes() {
            f(Offset::from_bytes(offset).unwrap() + start, true)?
        }
    }
    Ok(())
}


/// Call f(start + 0, is_cell_0), f(start + 1, is_cell_1), ..., f(start + size - 1, is_cell_size-1)
/// where is_cell_i is true if the i-th byte in the range does not contain an UnsafeCell.
fn iter_freeze_sensitive(
    cell_strategy: UnsafeCellStrategy,
    layout_strategy: LayoutStrategy,
    ptr_metadata: Option<PointerMeta<TreeBorrowsProvenance>>,
    start: Offset,
    size: Size,
    mut f: impl FnMut(Offset, bool) -> Result
) -> Result {
    match (cell_strategy, layout_strategy, ptr_metadata) {
        (UnsafeCellStrategy::Sized { bytes }, ..) => {
            act_on_bytes(bytes, start, size, f)?
        },
        (UnsafeCellStrategy::Slice { element }, LayoutStrategy::Slice(size, _), Some(PointerMeta::ElementCount(count))) => {
            for i in Int::ZERO..count {
                let offset = size * i;
                act_on_bytes(element, start + offset, size, &mut f)?
            };
        },
        (UnsafeCellStrategy::TraitObject { .. }, LayoutStrategy::TraitObject(_trait_name), Some(PointerMeta::VTablePointer(_ptr))) => {
            todo!("UnsafeCellStrategy::TraitObject non-freeze bytes")
        },
        (UnsafeCellStrategy::Tuple { head, tail }, LayoutStrategy::Tuple { head: TupleHeadLayout { end, .. }, tail: tail_layout }, _) => {
            act_on_bytes(head, start, size, &mut f)?;
            iter_freeze_sensitive(tail, tail_layout, ptr_metadata, start + end, size, f)?
        },
        _ => panic!("Invalid UnsafeCellStrategy, LayoutStrategy and PointerMeta combination"),
    };
    Ok(())
}

impl<T: Target> TreeBorrowsMemory<T> {
    /// Create a new node for a pointer (reborrow)
    fn reborrow(
        &mut self,
        ptr: Pointer<TreeBorrowsProvenance>,
        pointee_info: PointeeInfo,
        mutbl: Mutability,
        protected: Protected,
        frame_extra: &mut TreeBorrowsFrameExtra,
        vtable_lookup: impl Fn(ThinPointer<TreeBorrowsProvenance>) -> crate::lang::VTable + 'static,
    ) -> Result<ThinPointer<TreeBorrowsProvenance>> {
        let thin_ptr = ptr.thin_pointer;
        let (pointee_size, _align) = pointee_info.layout.compute_size_and_align(ptr.metadata, vtable_lookup);

        // Make sure the pointer is dereferenceable.
        self.mem.check_ptr(thin_ptr, pointee_size)?;
        // However, ignore the result of `check_ptr`: even if pointee_size is 0, we want to create a child pointer.
        let Some((alloc_id, parent_path)) = thin_ptr.provenance else {
            assert!(pointee_size.is_zero());
            // Pointers without provenance cannot access any memory, so giving them a new
            // tag makes no sense.
            return ret(thin_ptr);
        };

        let child_path = self.mem.allocations.mutate_at(alloc_id.0, |allocation| {
            let size = allocation.size();
            let offset = Offset::from_bytes(thin_ptr.addr - allocation.addr).unwrap();

            // Permission for the surrounding data of the pointee.  We allow lazily
            // writing to surrounding data if there is an `UnsafeCell` in the pointee.
            let (freeze_perm, nonfreeze_perm) = if mutbl == Mutability::Immutable {
                (Permission::Frozen, Permission::Cell)
            } else {
                (Permission::Reserved { conflicted: false }, Permission::ReservedIm)
            };
            let default_perm = if pointee_info.unsafe_cells.is_freeze_outside() { freeze_perm } else { nonfreeze_perm };
            let mut location_states = LocationState::new_list(default_perm, size);

            // Compute permissions
            iter_freeze_sensitive(pointee_info.unsafe_cells, pointee_info.layout, ptr.metadata, offset, pointee_size, |offset, frozen| {
                let permission = match mutbl {
                    // We only use `ReservedIm` for *unprotected* mutable references with interior mutability.
                    // If the reference is protected, we ignore the interior mutability.
                    // An example for why "Protected + Interior Mutability" is undesirable
                    // can be found in tooling/minimize/tests/ub/tree_borrows/protector/ReservedIm_spurious_write.rs.
                    Mutability::Mutable if !frozen && protected.no() => Permission::ReservedIm,
                    Mutability::Mutable => Permission::Reserved { conflicted: false },
                    Mutability::Immutable if !frozen => Permission::Cell,
                    Mutability::Immutable => Permission::Frozen,
                };

                location_states.set(offset.bytes(), LocationState {
                    accessed: Accessed::No, // This gets updated to `Accessed::Yes` if `allocation.extra.root.access(...)` runs.
                    permission,
                });
                Ok(())
            })?;

            // Create the new child node
            let child_node = Node {
                children: List::new(),
                location_states,
                protected,
            };

            // Add the new node to the tree
            let child_path = allocation.extra.root.add_node(parent_path, child_node);

            // If this is a non-zero-sized reborrow, perform read on the new child if needed, updating all nodes accordingly.
            if pointee_size.bytes() > 0 {
                iter_freeze_sensitive(pointee_info.unsafe_cells, pointee_info.layout, ptr.metadata, offset, pointee_size, |offset, _frozen| {
                    // We don't want to perform a read access on the non-frozen part if we have a shared reference,
                    // i.e. when we have a Cell permission.  For mutable references, the only difference between
                    // the ReservedIM and Reserved permissions is how resistant they are to foreign writes, so
                    // mutable references should have an implicit read access.
                    if location_states.get(offset.bytes()).unwrap().permission != Permission::Cell {
                        allocation.extra.root.access(Some(child_path), AccessKind::Read, offset, Offset::from_bytes_const(1))?
                    }
                    Ok(())
                })?
            }

            ret::<Result<Path>>(child_path)
        })?;

        // Track the new protector
        if protected.yes() { frame_extra.protectors.push((alloc_id, child_path)); }

        // Create the child pointer and return it
        ret(ThinPointer {
            provenance: Some((alloc_id, child_path)),
            ..thin_ptr
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
    fn ptr_reborrow_settings(ptr_type: PtrType, fn_entry: bool) -> Option<(Mutability, Protected, PointeeInfo)> {
        match ptr_type {
            PtrType::Ref { mutbl, pointee } if !pointee.unpin && mutbl == Mutability::Mutable => {
                // Mutable reference to pinning type: retagging is a NOP.
                None
            },
            PtrType::Ref { mutbl, pointee } => {
                let protected = if fn_entry { Protected::Strong } else { Protected::No };
                Some((mutbl, protected, pointee))
            },
            PtrType::Box { pointee } => {
                let protected = if fn_entry { Protected::Weak } else { Protected::No };
                Some((Mutability::Mutable, protected, pointee))
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
        vtable_lookup: impl Fn(ThinPointer<Self::Provenance>) -> crate::lang::VTable + 'static,
    ) -> Result<Pointer<Self::Provenance>> {
        ret(if let Some((mutbl, protected, pointee_info)) = Self::ptr_reborrow_settings(ptr_type, fn_entry) {
            self.reborrow(ptr, pointee_info, mutbl, protected, frame_extra, vtable_lookup)?.widen(ptr.metadata)
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
