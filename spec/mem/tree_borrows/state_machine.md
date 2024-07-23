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
}
```

Then we define the transition table.

```rust
impl TreeBorrowsAllocation {
    fn child_read(permission: Option<Permission>) -> Result<Option<Permission>> {
        let Some(permission) = permission else {
            throw_ub!("Tree Borrows: Child reading a pointer without permission")
        };

        ret(Some(
            match permission {
                Permission::Reserved => Permission::Reserved,
                Permission::Active => Permission::Active,
                Permission::Frozen => Permission::Frozen,
            }
        ))
    }

    fn child_write(permission: Option<Permission>) -> Result<Option<Permission>> {
        let Some(permission) = permission else {
            throw_ub!("Tree Borrows: Child writing a pointer without permission")
        };

        match permission {
            Permission::Reserved => ret(Some(Permission::Active)),
            Permission::Active => ret(Some(Permission::Active)),
            Permission::Frozen => throw_ub!("Tree Borrows: Child writing a pointer with the Frozen permission"),
        }
    }

    fn foreign_read(permission: Option<Permission>) -> Result<Option<Permission>> {
        let Some(permission) = permission else {
            return ret(permission)
        };

        ret(
            match permission {
                Permission::Reserved => Some(Permission::Reserved),
                Permission::Active => Some(Permission::Frozen),
                Permission::Frozen => Some(Permission::Frozen),
            }
        )
    }

    // FIXME: consider interior mutability
    fn foreign_write(_permission: Option<Permission>) -> Result<Option<Permission>> {
        ret(None)
    }

    fn permission_transition(
        &mut self,
        tag: BorTag,
        addr: Address,
        access_kind: AccessKind,
        node_relation: NodeRelation,
    ) -> Result {
        let Some(mut node) = self.tree.nodes.get(tag) else { throw_ub!("Tree Borrows: node not existed"); };

        let curr_permission = node.permissions.get(addr);

        let next_permission = match (node_relation, access_kind) {
            (NodeRelation::Foreign, AccessKind::Write) => Self::foreign_write(curr_permission)?,
            (NodeRelation::Foreign, AccessKind::Read) => Self::foreign_read(curr_permission)?,
            (NodeRelation::Child, AccessKind::Read) => Self::child_read(curr_permission)?,
            (NodeRelation::Child, AccessKind::Write) => Self::child_write(curr_permission)?,
        };

        if let Some(permission) = next_permission {
            node.permissions.insert(addr, permission);
        } else {
            node.permissions.remove(addr);
        }

        self.tree.nodes.insert(tag, node);
        ret(())
    }
}
```
