# Protected State Machine for Tree Borrows

The core of Tree Borrows is a state machine for each node and each location.
The protected state machine is more complicated than the unprotected state machine.
```rust
enum PermissionProt {
    /// Represents a shared reference to interior mutable data.
    Cell,
    /// The various flavours of `Reserved` correspond to a protected/noalias node where no writes happened yet
    Reserved { 
        local_read: bool,
        foreign_read: bool
    },
    /// Represents an activated (written to) mutable reference, i.e. it must actually be unique right now
    Unique,
    /// Represents a shared (immutable) reference
    Frozen { local_read: bool },
    /// Represents a dead reference
    DisabledForeignWrite,
}
```

Finally, we define the transition table.

```rust
impl PermissionProt {
    fn local_read(self) -> Result<PermissionProt> {
        ret(
            match self {
                PermissionProt::DisabledForeignWrite => throw_ub!("Tree Borrows: local read of a pointer with Disabled permission"),
                PermissionProt::Reserved { foreign_read, .. } => PermissionProt::Reserved { local_read: true, foreign_read },
                PermissionProt::Frozen { .. } => PermissionProt::Frozen { local_read: true },
                // Cell and Unique are unaffected.
                perm => perm,
            }
        )
    }

    fn local_write(self) -> Result<PermissionProt> {
        match self {
            PermissionProt::Cell => ret(PermissionProt::Cell),
            PermissionProt::Unique => ret(PermissionProt::Unique),
            PermissionProt::Reserved { foreign_read: false, .. } => ret(PermissionProt::Unique),
            PermissionProt::Reserved { foreign_read: true, .. } =>
                throw_ub!("Tree Borrows: writing to the local of a protected pointer with Conflicted Reserved permission"),
            PermissionProt::Frozen { .. } => throw_ub!("Tree Borrows: writing to the local of a pointer with Frozen permission"),
            PermissionProt::DisabledForeignWrite => throw_ub!("Tree Borrows: writing to the local of a pointer with Disabled permission"),
        }
    }

    fn foreign_read(self) -> Result<PermissionProt> {
        match self {
            PermissionProt::Cell => ret(PermissionProt::Cell),
            PermissionProt::Unique => throw_ub!("Tree Borrows: a protected pointer with Unique permission becomes Disabled"),
            PermissionProt::Reserved { local_read, .. } => ret(PermissionProt::Reserved { local_read, foreign_read: true }),
            // Frozen and Disabled are kept unchanged.
            perm => ret(perm),
        }
    }

    fn foreign_write(self) -> Result<PermissionProt> {
        match self {
            PermissionProt::Cell => ret(PermissionProt::Cell),
            PermissionProt::Frozen { local_read } | PermissionProt::Reserved { local_read, .. } if local_read =>
                    throw_ub!("Tree Borrows: a protected pointer with {} permission becomes Disabled", if matches!(self, PermissionProt::Frozen {..}) { "Frozen" } else { "Reserved" }),
            PermissionProt::Unique =>
                    throw_ub!("Tree Borrows: a protected pointer with Unique permission becomes Disabled"),

            // All other states become Disabled.
            _ => ret(PermissionProt::DisabledForeignWrite),
        }
    }

    fn transition(
        self,
        access_kind: AccessKind,
        node_relation: NodeRelation,
    ) -> Result<PermissionProt> {
        match (node_relation, access_kind) {
            (NodeRelation::Local, AccessKind::Read) => self.local_read(),
            (NodeRelation::Local, AccessKind::Write) => self.local_write(),
            (NodeRelation::Foreign, AccessKind::Read) => self.foreign_read(),
            (NodeRelation::Foreign, AccessKind::Write) => self.foreign_write(),
        }
    }

    fn init_access(self) -> Option<AccessKind> {
        // Everything except for `Cell` gets an initial read access.
        match self {
            PermissionProt::Cell => None,
            _ => Some(AccessKind::Read)
        }
    }

    /// When a protector is released, we transition to the unprotected state machine.
    fn into_unprotected(self) -> (PermissionUnprot, Option<AccessKind>) {
        match self {
            PermissionProt::Unique => (PermissionUnprot::Unique, Some(AccessKind::Write)),
            PermissionProt::Reserved { local_read: true, .. } => (PermissionUnprot::Reserved, Some(AccessKind::Read)),
            PermissionProt::Frozen { local_read: true } => (PermissionUnprot::Frozen, Some(AccessKind::Read)),
            PermissionProt::Reserved { local_read: false, .. } => (PermissionUnprot::Reserved, None),
            PermissionProt::Frozen { local_read: false } => (PermissionUnprot::Frozen, None),
            PermissionProt::DisabledForeignWrite => (PermissionUnprot::Disabled, None),

            PermissionProt::Cell => (PermissionUnprot::Cell, None),
        }
    }

    /// Protected nodes might block allocation, based on their permission.
    /// Specifically, they block allocation iff they would cause UB on a foreign write.
    fn blocks_deallocation(&self) -> bool {
        match self {
            PermissionProt::Unique => true,
            PermissionProt::Reserved { local_read, .. } | PermissionProt::Frozen { local_read } => *local_read,

            PermissionProt::DisabledForeignWrite => false,
            PermissionProt::Cell => false,
        }
        // TODO maybe instead do the following?
        // self.foreign_write().is_err()
    }
}

```
