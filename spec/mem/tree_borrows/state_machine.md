# State Machine for Tree Borrows

The core of Tree Borrows is a state machine for each node and each location.
Note that this presentation of Tree Borrows splits the protected from the unprotected state machine.
The protected state machine is presented in `state_machine_protected.md`

We first track the *permission* of each node to access each location.
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

Which permission we have depends on whether we are protected or not.
```rust
enum Permission {
    Unprot(PermissionUnprot),
    Prot(PermissionProt)
}
```


Finally, we define the transition table.

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

    fn init_access(self) -> Option<AccessKind> {
        // Everything except for `Cell` gets an initial read access.
        match self {
            PermissionUnprot::Cell => None,
            _ => Some(AccessKind::Read)
        }
    }
}


impl Permission {

    fn init_access(self) -> Option<AccessKind> {
        match self {
            Permission::Unprot(p) => p.init_access(),
            Permission::Prot(p) => p.init_access()
        }
    }

    fn transition(
        &mut self,
        access_kind: AccessKind,
        node_relation: NodeRelation,
        protected: bool,
    ) -> Result {
        match self {
            Permission::Unprot(p) => {
                assert!(!protected);
                *p = p.transition(access_kind, node_relation)?
            },
            Permission::Prot(p) => {
                assert!(protected);
                *p = p.transition(access_kind, node_relation)?
            }
        };
        Ok(())
    }

    fn unprotect(&mut self) -> Option<AccessKind> {
        match self {
            Permission::Unprot(_) => unreachable!(),
            Permission::Prot(p) => {
                let (new_perm, access) = p.into_unprotected();
                *self = Permission::Unprot(new_perm);
                access
            }
        }
    }

    fn blocks_deallocation(&self) -> bool {
        match self {
            Permission::Unprot(_) => false,
            Permission::Prot(p) => p.blocks_deallocation()
        }
    }

    fn select_based_on_protector(prot: Protected, p: (PermissionUnprot, PermissionProt)) -> Self {
        if prot.yes() {
            Permission::Prot(p.1)
        } else {
            Permission::Unprot(p.0)
        }
    }
}
```
