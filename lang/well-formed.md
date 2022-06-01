# MiniRust well-formedness requirements

The various syntactic constructs of MiniRust (types, functions, ...) come with well-formedness requirements: certain invariants need to be satisfied for this to be considered a well-formed program.
The idea is that for well-formed programs, the `step` function will never panic.
Those requirements are defined in this file.

Note that `check` functions for testing well-formedness return `Option<()>` rather than `bool` so that we can use `?`.

## Well-formed layouts and types

```rust
impl IntType {
    fn check(self) -> Option<()> {
        if !self.size.bytes().is_power_of_two() { return None; }
    }
}

impl Layout(self) {
    fn check(self) -> Option<()> {
        // Size must be a multiple of alignment.
        if self.size.bytes() % self.align.bytes() != 0 { return None; }
    }
}

impl Type {
    fn check(self) -> Option<()> {
        use Type::*;
        match self {
            Int(int_type) => {
                int_type.check()?;
            }
            Bool | RawPtr { .. } => (),
            Ref { pointee, .. } | Box { pointee } => {
                pointee.check()?;
            }
            Tuple { fields, size, align } => {
                // The fields must not overlap.
                // We check fields in the order of their (absolute) offsets.
                fields.sort_by_key(|(offset, type)| offset);
                let mut last_end = Size::ZERO;
                for (offset, type) in fields {
                    // Recursively check the field type.
                    type.check()?;
                    // And ensure it fits after the one we previously checked.
                    if offset < last_end { return None; }
                    last_end = offset.checked_add(type.size())?;
                }
                // And they must all fit into the size.
                if size < last_end { return None; }
            }
            Array { elem, count } => {
                elem.check()?;
                elem.size().checked_mul(count)?;
            }
            Union { fields, size } => {
                // These may overlap, but they must all fit the size.
                for (offset, type) in fields {
                    type.check()?;
                    if size < offset.checked_add(type.size())? { return None; }
                }
            }
            Enum { variants, size, .. } => {
                for variant in variants {
                    variant.check()?;
                    if size < variant.size() { return None; }
                }
            }
        }
    }
}

impl PlaceType {
    fn check(self) -> Option<()> {
        self.type.check()?;
        self.layout().check()?;
    }
}
```

## Well-formed expressions

```rust
impl Value {
    fn check(self, type: Type) -> Option<()> {
        // For now, we only support integer and boolean literals.
        match (self, type) {
            (Value::Int(i), Type::Int(int_type)) => {
                if !i.in_bounds(int_type.signed, int_type.size) { return None; }
            }
            (Value::Bool(_), Type::Bool) => (),
            _ => return None,
        }
    }
}

impl ValueExpr {
    fn check(self, locals: Map<Local, PlaceType>) -> Option<Type> {
        match self {
            Constant(value, type) => {
                value.check(type)?;
                Some(type)
            }
            Load { source, destructive: _ } => {
                let ptype = source.check(locals)?;
                Some(ptype.type)
            }
            Ref { target, align, mutbl } => {
                let ptype = target.check(locals)?;
                // If `align > ptype.align`, then this operation is "unsafe"
                // since the reference promises more alignment than what the place
                // guarantees. That is exactly what happens for references
                // to packed fields.
                let pointee = Layout { align, ..ptype.layout() };
                Some(Ref { mutbl, pointee })
            }
            AddrOf { target, mutbl } => {
                let ptype = target.check(locals)?;
                Some(RawPtr { mutbl });
            }
            UnOp { operator, operand } => {
                let operand = operand.check(locals)?;
                match operator {
                    Int(int_op, int_ty) => {
                        if !matches!(operand, Int(_)) { return None; }
                        Some(Int(int_ty))
                    }
                }
            }
            BinOp { operator, left, right } => {
                let left = left.check(locals)?;
                let right = right.check(locals)?;
                match operator {
                    Int(int_op, int_ty) => {
                        if !matches!(left, Int(_)) { return None; }
                        if !matches!(right, Int(_)) { return None; }
                        Some(Int(int_ty))
                    }
                    PtrOffset { inbounds: _ } => {
                        if !matches!(left, Ref { .. } | RawPtr { .. }) { return None; }
                        if !matches!(right, Int(_)) { return None; }
                        Some(left)
                    }
                }
            }
        }
    }
}

impl PlaceExpr {
    fn check(self, locals: Map<Local, PlaceType>) -> Option<PlaceType> {
        match self {
            Local(name) => locals.get(name),
            Deref { operand, align } => {
                let type = operand.check(locals)?;
                Some(PlaceType { type, align })
            }
            Field { root, field } => {
                let root = root.check(locals)?;
                let (offset, field_ty) = match root.type {
                    Tuple { fields, .. } => fields.get(field)?,
                    Union { fields, .. } => fields.get(field)?,
                    _ => return None,
                };
                // TODO: I am not sure that that this is a valid PlaceType
                // (specifically, that size is a multiple of align).
                Some(PlaceType {
                    align: root.align.restrict_for_offset(offset),
                    type: field_ty,
                })
            }
            Index { root, index } => {
                let root = root.check(locals)?;
                let index = index.check(locals)?;
                if !matches!(index, Int(_)) { return None; }
                let field_ty = match root.type {
                    Array { elem, .. } => elem,
                    _ => return None,
                };
                // We might be adding a multiple of `field_ty.size`, so we have to
                // lower the alignment compared to `root`. `restrict_for_offset`
                // is good for any multiple of that offset as well.
                // TODO: I am not sure that that this is a valid PlaceType
                // (specifically, that size is a multiple of align).
                Some(PlaceType {
                    align: root.align.restrict_for_offset(field_ty.size()),
                    type: field_ty,
                })
            }
        }
    }
}
```

## Well-formed functions and programs

When checking functions, we track for each program point the set of live locals (and their type) at that point.
To handle cyclic CFGs, we track the set of live locals at the beginning of each basic block.
When we first encounter a block, we add the locals that are live on the "in" edge; when we encounter a block the second time, we require the set to be the same.

```rust
impl Statement {
    /// This returns the adjusted live local mapping after the statement.
    fn check(
        self,
        mut live_locals: Map<Local, PlaceType>,
        func: Function
    ) -> Option<Map<Local, PlaceType>> {
        match self {
            Assign { destination, source } => {
                let left = destination.check(live_locals)?;
                let right = source.check(live_locals)?;
                if left.type != right { return None; }
                Some(locals)
            }
            Finalize { place } => {
                place.check(live_locals)?;
                Some(locals)
            }
            StorageLive(local) => {
                // Look up the type in the function, and add it to the live locals.
                // Fail if it already is live.
                locals.try_insert(local, func.locals.get(local)?)?;
                Some(locals)
            }
            StorageDead(local) => {
                locals.remove(local)?;
                Some(locals)
            }
        }
    }
}

impl Terminator {
    /// Returns the successor basic blocks that need to be checked next.
    fn check(self, live_locals: Map<Local, PlaceType>) -> Option<List<BbName>> {
        match self {
            Goto(block_name) => {
                Some(list![block_name])
            }
            If { condition, then_block, else_block } => {
                let type = condition.check(live_locals)?;
                if !matches!(type, Type::Bool) { return None; }
                Some(list![then_block, else_block])
            }
            // TODO: Call, Return
        }
    }
}
```

- TODO: define `check` for functions, programs
