# Niche claims

This file defines the `NicheClaim` type used by the niche-extension to Tree
Borrows. The extension makes writes-through-niche-bypass UB and so justifies
discriminant-elimination optimisations on niche-encoded enums. For background,
see [rust-lang/unsafe-code-guidelines#84](https://github.com/rust-lang/unsafe-code-guidelines/issues/84)
and the [proposal draft](https://hackmd.io/2__yv8jLR1yiMhnxHvU4Ig).

A claim is created at the language layer during place evaluation: a `Downcast(E, V)`
projection on a place appends a fresh `NicheClaim` to the place's
[`niche_claims`](../../lang/values.md) list. At retag (`AddrOf` or `Validate`)
the claim is consumed by the [retag clause](memory.md), which attaches it to a
synthetic intermediate tag `T_n` in the tree. In normal use only those synthetic
tags carry a non-empty list of claims; user-visible reborrow tags carry an empty
list.

```rust
/// A claim that, at `enum_addr` (absolute virtual address), the bytes decode
/// (via `enum_ty`'s discriminator) to `variant`.
///
/// The address is virtual; the tree-borrows layer converts to allocation-
/// relative byte offsets when attaching the claim to a tag and when checking it
/// against an access.
pub struct NicheClaim {
    /// Absolute virtual address at which `enum_ty` starts.
    /// We need an address (rather than a per-tag offset) because tags can span
    /// larger ranges than a single enum (e.g. an `&mut [T; N]` covers the whole
    /// array), and the same tag can be carried by pointers arithmetically
    /// shifted to elsewhere in the allocation. A claim is always about *one
    /// specific enum value at a specific place in memory*, so the position is
    /// recorded explicitly.
    pub enum_addr: Address,
    /// The enum type (`enum_of(V)` in the proposal). Used to drive the decoder
    /// and to identify which enum the claim is about -- claims survive transmutes
    /// to other types, and the decoder used at check time is always the one for
    /// `enum_ty`.
    pub enum_ty: lang::Type,
    /// The variant discriminant we claim the bytes decode to (`V` in the proposal).
    pub variant: Int,
}
```

While the claim is alive, any Foreign access through some tag `T_a` that touches
any byte of the claim's `niche_region` fires the niche check: the bytes (read
directly for the access's range, phantom-read through `T_n` for the rest of the
niche region) must decode via `enum_ty`'s discriminator to `variant`, or it is
UB. A passing Foreign Write discharges the claim; a passing Foreign Read retains
it.

The `niche_region` of an enum claim is the **set of bytes** that the
discriminator's decoder may read on some path that *ends in the claimed
variant*. Minirust's [`Discriminator`](../../lang/types.md) is a general
decision tree where each `Branch` node reads `value_type.size` bytes at some
`offset`, then dispatches to a child or `fallback`; different paths may read at
unrelated offsets, so the byte set is in general non-contiguous. Restricting to
paths that yield the claimed variant keeps the set as tight as possible -- bytes
on paths leading exclusively to other variants are irrelevant to confirming or
refuting *this* claim. If no path of the discriminator produces the claimed
variant we return `None` (the claim is unsatisfiable; this should be ruled out
by well-formedness on the enum type).

```rust
/// The set of byte offsets (within the enum's own coordinates) that the
/// decoder may read on some path of `discriminator` that ends in `value`.
/// `Some(Set::new())` if the path is trivially `Known(value)` (the decoder
/// reads nothing). `None` if no path of the discriminator produces `value`.
fn niche_region_of(discriminator: lang::Discriminator, value: Int) -> Option<Set<Offset>> {
    match discriminator {
        lang::Discriminator::Known(v) => if v == value { Some(Set::new()) } else { None },
        lang::Discriminator::Invalid => None,
        lang::Discriminator::Branch { offset, value_type, children, fallback } => {
            // Collect the union of byte sets across all sub-paths that reach
            // `value`. If no sub-path does, this branch contributes nothing.
            let mut bytes: Set<Offset> = Set::new();
            let mut any = false;
            for (_, child) in children.iter() {
                if let Some(sub) = niche_region_of(child, value) {
                    any = true;
                    for byte in sub { bytes.insert(byte); }
                }
            }
            if let Some(sub) = niche_region_of(fallback, value) {
                any = true;
                for byte in sub { bytes.insert(byte); }
            }
            if !any { return None; }
            // This branch's own bytes are read on every path that reaches
            // any descendant -- including the `value`-paths we found above.
            for i in Int::ZERO..value_type.size.bytes() {
                bytes.insert(offset + Size::from_bytes(i).unwrap());
            }
            Some(bytes)
        }
    }
}

impl NicheClaim {
    /// The set of byte offsets in **allocation** coordinates that the claim's
    /// discriminator may read on some path that ends in `self.variant`, given
    /// the allocation's base address.
    fn niche_region_in_alloc(self, alloc_addr: Address) -> Set<Offset> {
        let lang::Type::Enum { discriminator, .. } = self.enum_ty else {
            panic!("NicheClaim::niche_region_in_alloc: non-enum type");
        };
        let alloc_relative_enum_start = Offset::from_bytes(self.enum_addr - alloc_addr).unwrap();
        let bytes = niche_region_of(discriminator, self.variant).unwrap_or(Set::new());
        bytes.iter().map(|byte| alloc_relative_enum_start + byte).collect()
    }

    /// Decode the claim's enum from `alloc_data` using its discriminator,
    /// returning the discriminant value if decoding succeeds. Returns `None`
    /// on `Discriminator::Invalid` or on uninitialised tag bytes. This is
    /// the check the niche-extension runs against `self.variant`.
    fn decode<M: Memory>(self, alloc_addr: Address, alloc_data: List<AbstractByte<M::Provenance>>) -> Option<Int> {
        let lang::Type::Enum { discriminator, .. } = self.enum_ty else {
            panic!("NicheClaim::decode: non-enum type");
        };
        let alloc_relative_enum_start = self.enum_addr - alloc_addr;
        // The accessor never errors (we are reading from a local slice).
        lang::decode_discriminant::<M>(
            |off, sz| ret(alloc_data.subslice_with_length(
                alloc_relative_enum_start + off.bytes(),
                sz.bytes(),
            )),
            discriminator,
        ).unwrap()
    }
}
```

The `Node::access` method that drives a TB-level access folds the niche check
defined here into the same recursive tree walk: see [`tree.md`](tree.md). At
each visited foreign node, when `do_niche_check` is `true`, the walk decodes
the current bytes via this claim's discriminator (UB on mismatch), issues
phantom Local Reads through the niche tag for niche bytes outside the access
range (recursively, with `do_niche_check = false` to terminate), and on a
Foreign Write trigger discharges the claim.

See [§3.3 of the proposal][niche-check-proposal] for the rationale.

[niche-check-proposal]: https://hackmd.io/2__yv8jLR1yiMhnxHvU4Ig
