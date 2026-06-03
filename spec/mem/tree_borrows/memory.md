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
impl<T: Target> TreeBorrowsMemory<T> where Self: Memory<Provenance = TreeBorrowsProvenance> {
    /// Create a new node for a pointer (reborrow).
    ///
    /// If any of `niche_claims` overlap the new tag's pointee, the retag uses
    /// the two-child structure of the niche-extension: a single synthetic
    /// intermediate tag (`T_n` in the proposal) is created carrying all the
    /// surviving claims, and the user-visible tag (`T_c`) becomes its child.
    /// Claims whose niche bytes are disjoint from the new tag's pointee are
    /// silently dropped (the proposal's disjoint clause).
    fn reborrow(
        &mut self,
        ptr: ThinPointer<TreeBorrowsProvenance>,
        settings: ReborrowSettings,
        niche_claims: List<NicheClaim>,
        frame_extra: &mut TreeBorrowsFrameExtra,
    ) -> Result<ThinPointer<TreeBorrowsProvenance>> {
        let pointee_size = Size::from_bytes(settings.inside.len()).unwrap();

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
            // Prepare `permissions` which covers the *entire allocation*, not just
            // the part "inside" the pointer. This will initially not be marked as "accessed"
            // anywhere; we'll then do an access which will set that flag.
            let alloc_size = allocation.size();
            let offset = Offset::from_bytes(ptr.addr - allocation.addr).unwrap();

            // Filter niche_claims to those that have at least one niche byte
            // inside T_c's pointee for which T_c's `inside` permission at that
            // byte is writable. Bytes outside the pointee are inaccessible
            // through T_c, and bytes whose inside permission is read-only
            // (e.g. `Frozen` on a shared borrow) cannot violate the niche;
            // either way no T_n is needed. The disjoint clause and the gating
            // on shared borrows both fall out of this single check.
            let mut surviving_claims: List<NicheClaim> = List::new();
            let mut surviving_bytes: Set<Offset> = Set::new();
            for claim in niche_claims {
                // The niche region is in general a non-contiguous set of bytes:
                // minirust's `Discriminator` is a decision tree and different
                // paths may read at unrelated offsets. We take only the bytes
                // on paths that end in `claim.variant`.
                let niche_bytes = claim.niche_region_in_alloc(allocation.addr);
                let writable_overlap = niche_bytes.iter().any(|b| {
                    offset <= b && b < offset + pointee_size
                        && settings.inside.index_at(b.bytes() - offset.bytes()).is_writable()
                });
                if writable_overlap {
                    surviving_claims.push(claim);
                    for byte in niche_bytes { surviving_bytes.insert(byte); }
                }
            }

            // If any claims survived, build a single T_n carrying all of them
            // and parent T_c under it. T_n is unprotected (synthetic, no user
            // reference holds it), and uniformly `Reserved` everywhere -- bytes
            // inside the union of niche regions are initialised by the implicit
            // access below; bytes outside are irrelevant to the niche check and
            // never accessed through T_n.
            let current_parent_path = if surviving_claims.is_empty() {
                parent_path
            } else {
                let perm = Permission::Unprot(PermissionUnprot::Reserved);
                let inner_node = Node {
                    children: List::new(),
                    permissions: list![perm; alloc_size.bytes()],
                    protected: Protected::No,
                    niche_claims: surviving_claims,
                };
                assert!(inner_node.permissions.all(|x| x.matches_protector(inner_node.protected)));
                let inner_path = allocation.extra.root.add_node(parent_path, inner_node);
                // T_n's implicit init access at each byte in the union: a
                // single-byte Local Read, matching what
                // `Permission::Unprot(Reserved).init_access()` yields. We
                // iterate byte-by-byte since the union may be non-contiguous;
                // this mirrors the per-byte init loop for T_c below.
                if let Some(access) = perm.init_access() {
                    for byte in surviving_bytes {
                        allocation.extra.root.access::<Self>(
                            Some(inner_path), access, byte, Offset::from_bytes_const(1),
                            allocation.addr, allocation.data, true)?;
                    }
                }
                inner_path
            };

            // Create T_c (the user-visible tag) as a child of the deepest T_n (or of
            // the original parent if no T_n was created).
            let mut permissions = list![settings.outside; alloc_size.bytes()];
            if settings.inside.len() > 0 {
                permissions.write_subslice_at_index(
                    offset.bytes(),
                    settings.inside,
                );
            }

            // Create the new child node
            let child_node = Node {
                children: List::new(),
                permissions,
                protected: settings.protected,
                niche_claims: List::new(),
            };

            // Assert that the node's internal invariant is upheld.
            assert!(child_node.permissions.all(|x| x.matches_protector(child_node.protected)));

            // Add the new node to the tree
            let child_path = allocation.extra.root.add_node(current_parent_path, child_node);

            // Perform a read access on all bytes inside the pointee whose permission requires such an access.
            for (idx, perm) in settings.inside.iter().enumerate() {
                let idx = Int::from(idx); // FIXME: we need a version of `enumerate` that yields `Int`s.
                let idx = Size::from_bytes(idx).unwrap();
                if let Some(access) = perm.init_access() {
                    allocation.extra.root.access::<Self>(
                        Some(child_path), access, offset+idx,  Offset::from_bytes_const(1),
                        allocation.addr, allocation.data, true)?
                }
            }

            ret::<Result<Path>>(child_path)
        })?;

        // Track the new protector
        if settings.protected.yes() { frame_extra.protectors.push((alloc_id, child_path)); }

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
            let protector_accesses : List<_> = allocation.extra.root.access_node(path, |node| {
                assert!(node.protected.yes());
                // We get a list of pairs, and need to turn this into a pair of lists.
                let list_of_perms_and_accesses = node.permissions.map(|x| x.unprotect());
                // The list of unprotected permissions is stored back in the tree.
                node.permissions = list_of_perms_and_accesses.map(|(perm, _acc)| perm);
                // The resulting accesses are used further down.
                list_of_perms_and_accesses.map(|(_perm, acc)| acc)
            });

            if !allocation.live {
                // Looks like the protected memory got deallocated already. This is fine,
                // and expected for weak allocations. For strong allocations, this can e.g.
                // happen if the protector was zero-sized, or on interior mutable data.
                return ret(());
            }

            allocation.extra.root.release_protector(Some(path), &protector_accesses)
        })
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
            permissions: list![Permission::Unprot(PermissionUnprot::Unique); size.bytes()],
            protected: Protected::No,
            niche_claims: List::new(),
        };
        let path = Path::new();
        let extra = TreeBorrowsAllocationExtra { root };
        self.mem.allocate(kind, size, align, path, extra)
    }

    fn deallocate(&mut self, ptr: ThinPointer<Self::Provenance>, kind: AllocationKind, size: Size, align: Align) -> Result {
        self.mem.deallocate(ptr, kind, size, align, |extra, path, data| {
            // Check that ptr has the permission to write the entire allocation.
            extra.root.access::<Self>(
                Some(path), AccessKind::Write, Offset::ZERO, size,
                ptr.addr, data, true)?;

            // Check that allocation is not strongly protected.
            if extra.root.contains_strong_protector_preventing_deallocation() {
                throw_ub!("Tree Borrows: deallocating strongly protected allocation")
            }

            ret(())
        })
    }

    fn load(&mut self, ptr: ThinPointer<Self::Provenance>, len: Size, align: Align) -> Result<List<AbstractByte<Self::Provenance>>> {
        self.mem.load(ptr, len, align, |extra, path, offset, data| {
            // Check for aliasing violations.
            extra.root.access::<Self>(
                Some(path), AccessKind::Read, offset, len,
                ptr.addr - offset.bytes(), data, true)
        })
    }

    fn store(&mut self, ptr: ThinPointer<Self::Provenance>, bytes: List<AbstractByte<Self::Provenance>>, align: Align) -> Result {
        let size = Size::from_bytes(bytes.len()).unwrap();
        self.mem.store(ptr, bytes, align, |extra, path, offset, data| {
            // Check for aliasing violations.
            extra.root.access::<Self>(
                Some(path), AccessKind::Write, offset, size,
                ptr.addr - offset.bytes(), data, true)
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
        niche_claims: List<NicheClaim>,
        vtable_lookup: impl Fn(ThinPointer<Self::Provenance>) -> crate::lang::VTable + 'static,
    ) -> Result<Pointer<Self::Provenance>> {
        ret(if let Some(perms) = ReborrowSettings::new(ptr, ptr_type, fn_entry, vtable_lookup) {
            self.reborrow(ptr.thin_pointer, perms, niche_claims, frame_extra)?.widen(ptr.metadata)
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
