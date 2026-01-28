# Tree Borrows: Permission information for a reborrow

This file defines the settings for a reborrow: it says which permissions the new tag should get where.
The logic for applying those settings is separate (in the `reborrow` funcion).

```rust
struct ReborrowSettings {
    /// The permissions to set "inside" the data the reference points to (indexed
    /// related to the reference).
    inside: List<Permission>,
    /// The permission to set for the "outside" part.
    outside: Permission,
    /// Whether and which protector to set.
    protected: Protected,
}
```

Computing the permissions requires a "mask" indicating where all the `UnsafeCell` are.

```rust
/// Converts a list of cell locations into a "freeze mask",
/// where each `bool` indicates whether the byte is frozen (i.e., *outside* all cells).
fn cells_to_freeze_mask(cells: List<(Offset, Size)>, size: Size) -> List<bool> {
    let padded_front = std::iter::once((Offset::ZERO, Size::ZERO)).chain(cells.iter());
    let padded_back = cells.iter().chain(std::iter::once((size, Size::ZERO)));

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
    let mut mask = List::new();
    for (current, next) in padded_front.zip(padded_back) {
        // The `current` bytes are inside an UnsafeCell, and hence not frozen.
        mask.append(list![false; current.1.bytes()]);
        // The bytes until `next` are frozen.
        let no_cell_size = next.0.bytes() - (current.0 + current.1).bytes();
        mask.append(list![true; no_cell_size]);
    }
    assert!(mask.len() == size.bytes());
    mask
}

impl UnsafeCellStrategy {
    /// Converts a list of cell locations into a cell "mask",
    /// where each `bool` indicates whether the byte is inside a cell.
    fn freeze_mask(
        self,
        layout: LayoutStrategy,
        ptr_metadata: Option<PointerMeta<TreeBorrowsProvenance>>,
        vtable_lookup: impl Fn(ThinPointer<TreeBorrowsProvenance>) -> crate::lang::VTable + 'static,
    ) -> List<bool> {
        match (self, layout, ptr_metadata) {
            (UnsafeCellStrategy::Sized { cells }, LayoutStrategy::Sized(size, _align), ..) => {
                cells_to_freeze_mask(cells, size)
            },
            (UnsafeCellStrategy::Slice { element_cells } , LayoutStrategy::Slice(size, _align), Some(PointerMeta::ElementCount(count))) => {
                let element_mask = cells_to_freeze_mask(element_cells, size);
                let mut mask = List::new();
                for _ in Int::ZERO..count {
                    mask.append(element_mask);
                };
                mask
            },
            (UnsafeCellStrategy::TraitObject, LayoutStrategy::TraitObject(_trait_name), Some(PointerMeta::VTablePointer(ptr))) => {
                let vtable = vtable_lookup(ptr);
                cells_to_freeze_mask(vtable.cells, vtable.size)
            },
            (UnsafeCellStrategy::Tuple { head_cells, tail_cells }, LayoutStrategy::Tuple { head, tail }, _) => {
                let mut mask = cells_to_freeze_mask(head_cells, head.end);
                mask.append(tail_cells.freeze_mask(tail, ptr_metadata, vtable_lookup));
                mask
            },
            _ => panic!("Invalid LayoutStrategy and PointerMeta combination"),
        }
    }
}
```

Finally, the core operation is to compute a `ReborrowPermission` given a pointer and its type.

```rust
impl ReborrowSettings {
    /// Compute the permissions to be assigned when retagging the given pointer.
    /// `None` indicates that no retagging should happen.
    fn new(
        ptr: Pointer<TreeBorrowsProvenance>,
        ptr_type: PtrType,
        fn_entry: bool,
        vtable_lookup: impl Fn(ThinPointer<TreeBorrowsProvenance>) -> crate::lang::VTable + 'static,
    ) -> Option<Self> {
        let Some(pointee_info) = ptr_type.safe_pointee() else {
            return None;
        };
        if matches!(ptr_type, PtrType::Ref { mutbl: Mutability::Mutable, pointee } if !pointee.unpin) {
            // Mutable reference to pinning type: retagging is a NOP.
            // FIXME: with `UnsafePinned`, this should do proper per-byte tracking.
            return None;
        }

        // We protect upon function entry.
        let protected = if fn_entry {
            // Boxes are weakly protected, everything else strongly.
            match ptr_type {
                PtrType::Box { .. } => Protected::Weak,
                _ => Protected::Strong,
            }
        } else {
            Protected::No
        };

        // Helper to choose the correct permission, based on protection.
        let mk_perm = |unprot, prot| if protected.yes() { Permission::Prot(prot) } else { Permission::Unprot(unprot) };

        // Determine permissions.
        let no_cell_perm = match ptr_type {
            // Shared references are frozen.
            PtrType::Ref { mutbl: Mutability::Immutable, .. } => mk_perm(PermissionUnprot::Frozen, PermissionProt::Frozen { had_local_read: false }),
            // Mutable references and Boxes are reserved.
            _ => mk_perm(PermissionUnprot::Reserved, PermissionProt::Reserved { had_local_read: false, had_foreign_read: false }),
        };
        let cell_perm = match ptr_type {
            // Shared references to UnsafeCell use the "transparent" Cell permission.
            PtrType::Ref { mutbl: Mutability::Immutable, .. } => mk_perm(PermissionUnprot::Cell, PermissionProt::Cell),
            // Unprotected mutable references and boxes start in `ReservedIm`, but if they are protected we ignore the `Im`
            _ => mk_perm(PermissionUnprot::ReservedIm, PermissionProt::Reserved { had_local_read: false, had_foreign_read: false }),
        };
        let inside = pointee_info.unsafe_cells.freeze_mask(pointee_info.layout, ptr.metadata, vtable_lookup).map(|freeze|
            if freeze { no_cell_perm } else { cell_perm }
        );
        let outside = if pointee_info.freeze { no_cell_perm } else { cell_perm };

        Some(ReborrowSettings { protected, inside, outside })
    }
}
```
