# State Machine for Tree Borrows

The core of Tree Borrows is a state machine for each node and each location.

We first track the *permission* of each node to access each location.
```rust
enum Permission {
    /// Represents a two-phase borrow during its reservation phase
    Reserved {
        /// Indicates whether there was a foreign read.
        conflicted: bool,
    },
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

In addition, we also need to track whether a location has already been accessed with a pointer corresponding to this node.

```rust
enum Accessed {
    /// This address has been accessed (read, written, or the initial implicit read upon retag)
    /// with this borrow tag.
    Yes,
    /// This address has not yet been accessed with this borrow tag. We still track how foreign
    /// accesses affect the current permission so that on the first access, we start in the right state.
    No,
}
```

Then we define the per-location state tracked by Tree Borrows.
```rust
struct LocationState {
    accessed: Accessed,
    permission: Permission,
}
```

Finally, we define the transition table.

```rust
impl Permission {
    fn default(mutbl: Mutability, pointee: PointeeInfo, protected: Protected) -> Permission {
        match mutbl {
            // We only use `ReservedIm` for *unprotected* mutable references with interior mutability.
            // If the reference is protected, we ignore the interior mutability.
            // An example for why "Protected + Interior Mutability" is undesirable
            // can be found in tooling/minimize/tests/ub/tree_borrows/protector/ReservedIm_spurious_write.rs.
            Mutability::Mutable if !pointee.freeze && protected.no() => Permission::ReservedIm,
            Mutability::Mutable => Permission::Reserved { conflicted: false },
            Mutability::Immutable if pointee.freeze => Permission::Frozen,
            Mutability::Immutable => panic!("Permission::default: interior-mutable shared reference")
        }
    }

    fn local_read(self) -> Result<Permission> {
        ret(
            match self {
                Permission::Disabled => throw_ub!("Tree Borrows: local read of a pointer with Disabled permission"),
                // All other states are kept unchanged.
                perm => perm,
            }
        )
    }

    fn local_write(self, protected: bool) -> Result<Permission> {
        match self {
            Permission::Reserved { conflicted: true } if protected =>
                throw_ub!("Tree Borrows: writing to the local of a protected pointer with Conflicted Reserved permission"),
            Permission::Frozen => throw_ub!("Tree Borrows: writing to the local of a pointer with Frozen permission"),
            Permission::Disabled => throw_ub!("Tree Borrows: writing to the local of a pointer with Disabled permission"),
            _ => ret(Permission::Unique),
        }
    }

    fn foreign_read(self, protected: bool) -> Result<Permission> {
        match self {
            Permission::Unique if protected  => ret(Permission::Disabled),
            Permission::Reserved { .. } if protected => ret(Permission::Reserved { conflicted: true }),
            Permission::Unique => ret(Permission::Frozen),
            // All other states are kept unchanged.
            perm => ret(perm),
        }
    }

    fn foreign_write(self) -> Result<Permission> {
        match self {
            Permission::ReservedIm => ret(Permission::ReservedIm),
            // All other states become Disabled.
            _ => ret(Permission::Disabled),
        }
    }

    fn transition(
        self,
        access_kind: AccessKind,
        node_relation: NodeRelation,
        protected: bool,
    ) -> Result<Permission> {
        match (node_relation, access_kind) {
            (NodeRelation::Local, AccessKind::Read) => self.local_read(),
            (NodeRelation::Local, AccessKind::Write) => self.local_write(protected),
            (NodeRelation::Foreign, AccessKind::Read) => self.foreign_read(protected),
            (NodeRelation::Foreign, AccessKind::Write) => self.foreign_write(),
        }
    }
}

impl Accessed {
    fn transition(self, node_relation: NodeRelation) -> Accessed {
        // A node is "accessed" once any of its children gets accessed.
        match node_relation {
            NodeRelation::Foreign => self,
            NodeRelation::Local => Accessed::Yes,
        }
    }
}

impl LocationState {
    /// Create a location state list:
    /// all locations share the given permission.
    fn new_list(permission: Permission, len: Size) -> List<LocationState> {
        let mut location_states = List::new();
        for _ in Int::ZERO..len.bytes() {
            location_states.push(LocationState {
                accessed: Accessed::No,
                permission,
            });
        }

        location_states
    }

    fn transition(
        &mut self,
        access_kind: AccessKind,
        node_relation: NodeRelation,
        protected: bool,
    ) -> Result {
        let old_perm = self.permission;
        self.permission = old_perm.transition(access_kind, node_relation, protected)?;
        self.accessed = self.accessed.transition(node_relation);

        // Protected nodes may never transition to "Disabled, Accessed". That is UB.
        if self.permission == Permission::Disabled && protected && self.accessed == Accessed::Yes {
            // This is UB, make sure to show a somewhat specific error.
            match old_perm {
                Permission::Disabled => panic!("Impossible state combination: Accessed + Protected + Disabled"),
                Permission::ReservedIm => panic!("Impossible state combination: Accessed + Protected + ReservedIm"),
                Permission::Unique => throw_ub!("Tree Borrows: a protected pointer with Unique permission becomes Disabled"),
                Permission::Frozen => throw_ub!("Tree Borrows: a protected pointer with Frozen permission becomes Disabled"),
                Permission::Reserved { .. } => throw_ub!("Tree Borrows: a protected pointer with Reserved permission becomes Disabled"),
            }
        }

        ret(())
    }
}
```
