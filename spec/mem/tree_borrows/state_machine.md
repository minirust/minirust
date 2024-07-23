# State Machine for Tree Borrows

The core of Tree Borrows is a state machine for each node and each location.
We call the state a *Permission*.

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
impl Node {
    fn child_read(permission: Permission) -> Result<Permission> {
        ret(
            match permission {
                Permission::Reserved => Permission::Reserved,
                Permission::Active => Permission::Active,
                Permission::Frozen => Permission::Frozen,
                Permission::Disabled => throw_ub!("Tree Borrows: Child reading a pointer without permission"),
            }
        )
    }

    fn child_write(permission: Permission) -> Result<Permission> {
        match permission {
            Permission::Reserved => ret(Permission::Active),
            Permission::Active => ret(Permission::Active),
            Permission::Frozen => throw_ub!("Tree Borrows: Child writing a pointer with the Frozen permission"),
            Permission::Disabled => throw_ub!("Tree Borrows: Child writing a pointer without permission"),
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

    // FIXME: consider interior mutability
    fn foreign_write(_permission: Permission) -> Result<Permission> {
        ret(Permission::Disabled)
    }

    fn permission_transition(
        curr_permission: Permission,
        access_kind: AccessKind,
        node_relation: NodeRelation,
    ) -> Result<Permission> {
        match (node_relation, access_kind) {
            (NodeRelation::Foreign, AccessKind::Write) => Self::foreign_write(curr_permission),
            (NodeRelation::Foreign, AccessKind::Read) => Self::foreign_read(curr_permission),
            (NodeRelation::Child, AccessKind::Read) => Self::child_read(curr_permission),
            (NodeRelation::Child, AccessKind::Write) => Self::child_write(curr_permission),
        }
    }
}
```
