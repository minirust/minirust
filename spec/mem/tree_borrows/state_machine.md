# State Machine for Tree Borrows

The core of Tree Borrows is a state machine for each node and each location.

We first track the *permission* of each node to access each location.
```rust
enum Permission {
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

In addition, we also need to track whether a location has already been accessed.

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

Then we define the state for the Tree Borrows state machine.
```rust
struct LocationState {
    accessed: Accessed,
    permission: Permission,
}
```

Finally, we define the transition table.

```rust
impl Permission {
    fn child_read(self) -> Result<Permission> {
        ret(
            match self {
                Permission::Reserved => Permission::Reserved,
                Permission::Active => Permission::Active,
                Permission::Frozen => Permission::Frozen,
                Permission::Disabled => throw_ub!("Tree Borrows: Child reading a pointer with Disabled permission"),
            }
        )
    }

    fn child_write(self) -> Result<Permission> {
        match self {
            Permission::Reserved => ret(Permission::Active),
            Permission::Active => ret(Permission::Active),
            Permission::Frozen => throw_ub!("Tree Borrows: Child writing a pointer with the Frozen permission"),
            Permission::Disabled => throw_ub!("Tree Borrows: Child writing a pointer with Disabled permission"),
        }
    }

    fn foreign_read(self) -> Result<Permission> {
        ret(
            match self {
                Permission::Reserved => Permission::Reserved,
                Permission::Active => Permission::Frozen,
                Permission::Frozen => Permission::Frozen,
                Permission::Disabled => Permission::Disabled,
            }
        )
    }

    // FIXME: consider interior mutability
    fn foreign_write(self) -> Result<Permission> {
        ret(Permission::Disabled)
    }

    fn transition(
        self,
        access_kind: AccessKind,
        node_relation: NodeRelation,
    ) -> Result<Permission> {
        match (node_relation, access_kind) {
            (NodeRelation::Foreign, AccessKind::Write) => self.foreign_write(),
            (NodeRelation::Foreign, AccessKind::Read) => self.foreign_read(),
            (NodeRelation::Child, AccessKind::Read) => self.child_read(),
            (NodeRelation::Child, AccessKind::Write) => self.child_write(),
        }
    }
}

impl LocationState {
    fn transition(
        self,
        access_kind: AccessKind,
        node_relation: NodeRelation,
    ) -> Result<LocationState> {
        let permission = self.permission.transition(access_kind, node_relation)?;
        ret(LocationState { permission, accessed: Accessed::Yes })
    }
}
```
