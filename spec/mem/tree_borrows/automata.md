# State Machine For the Tree Borrows

The core of the Tree Borrows is a state machine. We track a state machine for each node on the tree. We call the state as *Permission*.

```rust
pub enum Permission {
    /// Represents a two-phase borrow during its reservation phase
    Reserved, 
    /// Represents an activated (written to) mutable reference
    Active,
    /// Represents a shared (immutable) reference
    Frozen,
    /// Represents a dead reference
    Disabled, 
}
```

Then we define the transition table.

```rust
impl TreeBorrowsAllocation {
    fn child_read(permission: Permission) -> Result<Permission> {
        match permission {
            Permission::Reserved => ret(Permission::Reserved),
            Permission::Active => ret(Permission::Active),
            Permission::Frozen => ret(Permission::Frozen),
            Permission::Disabled => throw_ub!("Tree Borrows: Child reading a pointer with the Disabled permission"),
        }
    }

    fn child_write(permission: Permission) -> Result<Permission> {
        match permission {
            Permission::Reserved => ret(Permission::Active),
            Permission::Active => ret(Permission::Active), 
            Permission::Frozen => throw_ub!("Tree Borrows: Child writing a pointer with the Frozen permission"), 
            Permission::Disabled => throw_ub!("Tree Borrows: Child writing a pointer with the Disabled permission"),
        }
    }

    fn foreign_read(permission: Permission) -> Result<Permission> {
        ret(
            match permission {
                Permission::Reserved => Permission::Reserved,
                Permission::Active => Permission::Frozen,
                Permission::Frozen => Permission::Frozen,
                Permission::Disabled => Permission::Disabled,
            }
        )
    }

     fn foreign_write(permission: Permission) -> Result<Permission> {
        ret(
            match permission {
                Permission::Reserved => Permission::Disabled,
                Permission::Active => Permission::Disabled,
                Permission::Frozen => Permission::Disabled,
                Permission::Disabled => Permission::Disabled,
            }
        )
    }

    fn permission_transition(curr: Permission, access_kind: AccessKind, node_relation: NodeRelation) -> Result<Permission> {
        match (access_kind, node_relation) {
            (AccessKind::Read, NodeRelation::Child) => Self::child_read(curr),
            (AccessKind::Write, NodeRelation::Child) => Self::child_write(curr),
            (AccessKind::Read, NodeRelation::Foreign) => Self::foreign_read(curr),
            (AccessKind::Write, NodeRelation::Foreign) => Self::foreign_write(curr),
        }
    }
}
```
