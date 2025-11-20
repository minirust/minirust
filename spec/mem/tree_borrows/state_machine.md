# State Machine for Tree Borrows

The core of Tree Borrows is a state machine for each node (reference) and each location.
The states of this state machine are called `Permission` since they regulate what each reference is _permitted_ to do.
Note that this presentation of Tree Borrows splits the protected from the unprotected state machine.
See `state_machine_protected.md` for the protected state machine, and `state_machine_unprotected.md` for the unprotected one.

The `Permission` just type just tracks which of the two state machines we are currrently using, and delegates the everything appropiately.

```rust
enum Permission {
    Unprot(PermissionUnprot),
    Prot(PermissionProt)
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
    ) -> Result {
        match self {
            Permission::Unprot(p) => {
                *p = p.transition(access_kind, node_relation)?
            },
            Permission::Prot(p) => {
                *p = p.transition(access_kind, node_relation)?
            }
        };
        Ok(())
    }

    /// Switches from the protected to the unprotected state machine on protector end.
    /// Also defines the protector end action.
    fn unprotect(self) -> (Permission, Option<AccessKind>) {
        match self {
            // This method is only called on protected nodes.
            Permission::Unprot(_) => unreachable!(),
            Permission::Prot(p) => {
                let (new_perm, access) = p.into_unprotected();
                (Permission::Unprot(new_perm), access)
            }
        }
    }

    /// Strongly protected nodes can block deallocation, based on their permission.
    /// Specifically, they block allocation iff they would cause UB on a foreign write,
    /// that is, if they have been locally accessed ("used"), with an exception for `Cell`.
    /// The check for whether the protector is actually strong happens elsewhere.
    fn prevents_deallocation(&self) -> bool {
        match self {
            Permission::Unprot(_) => false,
            Permission::Prot(p) => p.prevents_deallocation()
        }
    }
}
```
