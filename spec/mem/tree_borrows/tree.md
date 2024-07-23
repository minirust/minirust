# Tree Structure in Tree Borrows

The core data structure of the Tree Borrows is a *Tree*. We use a tree to track reborrows in each allocation. The tree consists of several nodes, each corresponding to a borrow tag (i.e., a reborrow operation). Each node has a parent and a list of children. Additionally, we track each tag's permissions on memory locations (bytes) within the allocation.

```rust
pub struct Tree {
    root_tag: BorTag, 
    /// Map a borrow tag to a node
    nodes: Map<BorTag, Node>,
}

pub struct Node {
    /// Borrow tag of the parent node
    parent: Option<BorTag>, 
    /// Borrow tags of the children node
    children: List<BorTag>,
    /// Permission for each location
    permissions: List<(Accessed, Permission)>,
}

impl Tree {
    /// Insert a node into the tree
    fn insert_node(&mut self, tag: BorTag, node: Node) {
        self.nodes.insert(tag, node);
    }
}
```

During each memory access, we update permissions according to the state machine defined in [state_machine.md](state_machine.md). When a node is accessed, each node in the tree can be divided into two disjoint sets: *Child* and *Foreign*, based on its relative position to the accessed node. The Child set includes the node itself and all its descendants, while the Foreign set contains all other nodes.

```rust
pub enum NodeRelation {
    Child, 
    Foreign,
}
```

The permission update depends on both the node relation and the type of access operation.

```rust
pub enum AccessKind {
    Read, 
    Write,
}
```

Then we implement how to actually update and check permissions in a Tree during each memory access.

```rust
impl Tree {
    /// Recusively do permission transition on the `curr` node and all its descendants.
    /// `base` means the node that is actually memory accessed.
    /// This method will throw UBs, representing the undefined behavior captured by Tree Borrows.
    /// The return value indicated whether the `access_tag` is a descendant of the `curr_tag`.
    fn node_access(
        &mut self, 
        curr_tag: BorTag,
        access_kind: AccessKind,
        offset_in_alloc: Size,
        size: Size,
        access_tag: BorTag,
    ) -> Result<bool> {
        let Some(mut node) = self.nodes.get(curr_tag) else { throw_ub!("Tree Borrows: node not existed"); };

        // Flag to indicate whether `acccess_tag`is a child of the `curr_tag`
        let mut is_child = curr_tag == access_tag;

        for child_tag in node.children {
            is_child |= self.node_access(child_tag, access_kind, offset_in_alloc, size, access_tag)?;
        }

        // Indicates whether the `access_tag` is a child or foreign
        // *from the perspective of the `curr_tag`*
        let node_relation = if is_child { NodeRelation::Child } else { NodeRelation::Foreign };

        let offset_start = offset_in_alloc.bytes();

        for offset in offset_start..offset_start + size.bytes() {
            let (_, curr_permission) = node.permissions[offset];
            let next_permission = curr_permission.transition(access_kind, node_relation)?;
            node.permissions.set(offset, (Accessed::Yes, next_permission));
        }

        self.nodes.insert(curr_tag, node);

        ret(is_child)
    }

    /// Recusively do permission transition starting from the root node.
    /// This method will throw UBs, representing the undefined behavior captured by Tree Borrows.
    fn access(
        &mut self, 
        base: BorTag, 
        access_kind: AccessKind,
        offset_in_alloc: Size,
        size: Size
    ) -> Result {
        // Each node is a descendant of the root node.
        self.node_access(self.root_tag, access_kind, offset_in_alloc, size, base)?;
        ret(())
    }
}
```
