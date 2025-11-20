# Unprotected State Machine for Tree Borrows

The states of the unprotected state machine are given by `PermissionUnprot`.

```rust
enum PermissionUnprot {
    /// Represents a shared reference to interior mutable data.
    Cell,
    /// Represents a two-phase borrow during its reservation phase
    Reserved,
    /// Represents a interior mutable two-phase borrow during its reservation phase
    ReservedIm,
    /// Represents an activated (written to) mutable reference, i.e. it must actually be unique right now
    Unique,
    /// Represents a shared (immutable) reference
    Frozen,
    /// Represents a dead reference
    Disabled,
}
```

The state machine transition table is given by these four functions.
When they return `Err` / signal UB, this means the state machine got stuck.

```rust
impl PermissionUnprot {
    fn local_read(self) -> Result<PermissionUnprot> {
        ret(
            match self {
                PermissionUnprot::Disabled => throw_ub!("Tree Borrows: Uniqueness violation: local read after foreign write"),
                // All other states are kept unchanged.
                perm => perm,
            }
        )
    }

    fn local_write(self) -> Result<PermissionUnprot> {
        match self {
            PermissionUnprot::Frozen => throw_ub!("Tree Borrows: Read-only violation: local write to a read-only reference (shared, or mutable after a foreign write)"),
            PermissionUnprot::Disabled => throw_ub!("Tree Borrows: Uniqueness violation: local write after foreign write"),
            PermissionUnprot::Cell => ret(PermissionUnprot::Cell),
            _ => ret(PermissionUnprot::Unique),
        }
    }

    fn foreign_read(self) -> Result<PermissionUnprot> {
        match self {
            PermissionUnprot::Reserved => ret(PermissionUnprot::Reserved),
            PermissionUnprot::Unique => ret(PermissionUnprot::Frozen),
            // All other states are kept unchanged.
            perm => ret(perm),
        }
    }

    fn foreign_write(self) -> Result<PermissionUnprot> {
        match self {
            PermissionUnprot::Cell => ret(PermissionUnprot::Cell),
            PermissionUnprot::ReservedIm => ret(PermissionUnprot::ReservedIm),
            // All other states become Disabled.
            _ => ret(PermissionUnprot::Disabled),
        }
    }

    fn transition(
        self,
        access_kind: AccessKind,
        node_relation: NodeRelation,
    ) -> Result<PermissionUnprot> {
        match (node_relation, access_kind) {
            (NodeRelation::Local, AccessKind::Read) => self.local_read(),
            (NodeRelation::Local, AccessKind::Write) => self.local_write(),
            (NodeRelation::Foreign, AccessKind::Read) => self.foreign_read(),
            (NodeRelation::Foreign, AccessKind::Write) => self.foreign_write(),
        }
    }
}
```

When a new node is created, it causes an implicit access, usually an implicit read.
This is so the optimizer can insert reads the moment a reference is created.
Note that some nodes do not cause an implicit read, this depends on the permission, and is defined by the `init_access` function.

```rust
impl PermissionUnprot {
    fn init_access(self) -> Option<AccessKind> {
        // Everything except for `Cell` gets an initial read access.
        match self {
            PermissionUnprot::Cell => None,
            _ => Some(AccessKind::Read)
        }
    }
}

```
