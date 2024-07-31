# Tree Structure in Tree Borrows

The core data structure of Tree Borrows is a *tree*, with a state machine in each node.
We use a tree to track reborrows in each allocation; each reborrow adds a new node to the tree.
The per-node state machine is defined in [state_machine.md](state_machine.md).

Structurally, we use the usual function representation of a tree: we store a list of children.

```rust
struct Node {
    children: List<Node>,
    /// State for each location
    location_states: List<LocationState>,
}
```

During each memory access, we update states according to the state machine.
When a node is accessed, each node in the tree can be divided into two disjoint sets: *child nodes* and *foreign nodes*, based on its relative position to the accessed node.
The *child* set includes the node itself and all its descendants, while the *foreign* set contains all other nodes.

```rust
enum NodeRelation {
    Child, 
    Foreign,
}
```

The state transition depends on both the node relation and the type of access operation.

```rust
enum AccessKind {
    Read, 
    Write,
}
```

Then we implement how to actually update and check states in a Tree during each memory access.

```rust
impl Node {
    /// Perform state transition on all locations of `self`.
    /// The `node_relation` indicates whether the accessed node is a child or foreign
    /// *from the perspective of `self`*.
    fn transition(
        &mut self, 
        node_relation: NodeRelation,
        access_kind: AccessKind,
        offset_in_alloc: Size,
        size: Size
    ) -> Result {
        let offset_start = offset_in_alloc.bytes();
        for offset in offset_start..offset_start + size.bytes() {
            self.location_states.mutate_at(offset, |location_state|{
                location_state.transition(access_kind, node_relation)
            })?;
        }

        ret(())
    }

    /// Recusively do state transition on `self` node and all its descendants.
    /// `path` is the path from `self` to the accessed node.
    /// `path` is None when the accessed node is not a descendant of `self`.
    ///
    /// This method will throw UBs, representing the undefined behavior captured by Tree Borrows.
    fn access(
        &mut self,
        path: Option<Path>, // self -> child
        access_kind: AccessKind,
        offset_in_alloc: Size,
        size: Size,
    ) -> Result {
        // Indicates whether the accessed node is a child or foreign
        // *from the perspective of `self`*.
        //
        // If `self` is an ancestor of the accessed node, the accessed node is a child of `self`.
        // Otherwise, the accessed node is a `foreign` of `self`.
        let node_relation = if path.is_some() { NodeRelation::Child } else { NodeRelation::Foreign };

        // Perform state transition on `self`.
        self.transition(node_relation, access_kind, offset_in_alloc, size)?;

        for child_id in Int::ZERO..self.children.len() {
            // If the path starts with this child, do a child access with the path shortened by the first element.
            // Otherwise, do a child access with the path being None (i.e., child is not an ancestor of accessed node)
            let sub_path = match path.and_then(|p| p.split_first()) {
                Some((head, tail)) if head == child_id => Some(tail),
                _ => None,
            };

            self.children.mutate_at(child_id, |child| {
                child.access(sub_path, access_kind, offset_in_alloc, size)
            })?;
        }

        ret(())
    }
}
```

We also implement some helper methods for manipulating Trees.

```rust
impl Node {
    /// Add a new child node to the tree whose root is self
    /// `path` is the path from `self` to the parent of the `node`.
    ///
    /// Return the path from `self` to the `node`
    fn add_node(&mut self, parent_path: Path, child: Node) -> Result<Path> {
        // `sub_root_id` indicates which node to access next; call it the sub-root.
        // `sub_path` is the path from the sub-root to the child
        let Some((sub_root_id, sub_path)) = parent_path.split_first() else {
            // If this is where we want to insert, we are done.
            let child_idx = self.children.len();
            self.children.push(child);
            return ret(list![child_idx]);
        };

        // If `self` is a leaf and the path keeps going, then the path is invalid.
        if self.children.len() == Int::ZERO {
            panic!("Node::add_node: invalid parent path");
        }

        // Find the right child, and add the node there.
        self.children.mutate_at(sub_root_id, |sub_root| {
            let mut child_path = sub_root.add_node(sub_path, child)?;
            // We got a path from `sub_root` to the new child; update it to start at `self`.
            child_path.push_front(sub_root_id);
            ret(child_path)
        })
    }

}
```
