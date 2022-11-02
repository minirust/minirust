# MiniRust well-formedness requirements

The various syntactic constructs of MiniRust (types, functions, ...) come with well-formedness requirements: certain invariants need to be satisfied for this to be considered a well-formed program.
The idea is that for well-formed programs, the `step` function will never panic.
Those requirements are defined in this file.

Note that `check` functions for testing well-formedness return `Option<()>` rather than `bool` so that we can use `?`.
We use the following helper function to convert Boolean checks into this form.

```rust
fn ensure(b: bool) -> Option<()> {
    if !b { throw!(); }
}
```

## Well-formed layouts and types

```rust
impl IntType {
    fn check(self) -> Option<()> {
        ensure(self.size.bytes().is_power_of_two())?;
    }
}

impl Layout {
    fn check(self) -> Option<()> {
        // Nothing to check here.
        // In particular, we do *not* require that size is a multiple of align!
        // To represent e.g. the PlaceType of an `i32` at offset 0 in a
        // type that is `align(16)`, we have to be able to talk about types
        // with size 4 and alignment 16.
    }
}

impl PtrType {
    fn check(self) -> Option<()> {
        match self {
            PtrType::Raw => (),
            PtrType::Ref { pointee, mutbl: _ } | PtrType::Box { pointee } => {
                pointee.check()?;
            }
        }
    }
}

impl Type {
    fn check<Info: MemoryInfo>(self) -> Option<()> {
        use Type::*;
        match self {
            Int(int_type) => {
                int_type.check()?;
            }
            Bool => (),
            Pointer(ptr_type) {
                ptr_type.check()?;
            }
            Tuple { fields, size, align } => {
                // The fields must not overlap.
                // We check fields in the order of their (absolute) offsets.
                fields.sort_by_key(|(offset, _ty)| offset);
                let mut last_end = Size::ZERO;
                for (offset, ty) in fields {
                    // Recursively check the field type.
                    ty.check::<Info>()?;
                    // And ensure it fits after the one we previously checked.
                    ensure(offset >= last_end)?;
                    last_end = offset.checked_add(ty.size::<Info>())?;
                }
                // And they must all fit into the size.
                ensure(size >= last_end)?;
            }
            Array { elem, count } => {
                elem.check::<Info>()?;
                elem.size::<Info>().checked_mul(count)?;
            }
            Union { fields, size, chunks } => {
                // The fields may overlap, but they must all fit the size.
                for (offset, ty) in fields {
                    ty.check::<Info>()?;
                    ensure(size >= offset.checked_add(ty.size::<Info>())?)?;

                    // And it must fit into one of the chunks.
                    ensure(chunks.into_iter().any(|(chunk_offset, chunk_size)| {
                        chunk_offset <= offset
                            && offset + ty.size::<Info>() <= chunk_offset + chunk_size
                    }))?;
                }
                // The chunks must be sorted in their offsets and disjoint.
                // FIXME: should we relax this and allow arbitrary chunk order?
                let mut last_end = Size::ZERO;
                for (offset, size) in chunks {
                    ensure(offset >= last_end)?;
                    last_end = offset.checked_add(size)?;
                }
                // And they must all fit into the size.
                ensure(size >= last_end)?;
            }
            Enum { variants, size, tag_encoding: _ } => {
                for variant in variants {
                    variant.check::<Info>()?;
                    ensure(size >= variant.size::<Info>())?;
                }
            }
        }
    }
}

impl PlaceType {
    fn check<Info: MemoryInfo>(self) -> Option<()> {
        self.ty.check::<Info>()?;
        self.layout::<Info>().check()?;
    }
}
```

## Well-formed expressions

```rust
impl<M: Memory> Value<M> {
    /// Check that the value has the expected type.
    /// Assumes that `ty` has already been checked.
    fn check(self, ty: Type) -> Option<()> {
        // For now, we only support integer and boolean literals, and arrays/tuples.
        // TODO: add more.
        match (self, ty) {
            (Value::Int(i), Type::Int(int_type)) => {
                ensure(i.in_bounds(int_type.signed, int_type.size))?;
            }
            (Value::Bool(_), Type::Bool) => (),
            (Value::Tuple(values), Type::Tuple { fields }) => {
                ensure(values.len() == fields.len())?;
                for (val, (_offset, ty)) in values.iter().zip(fields.iter()) {
                    val.check(ty)?;
                }
            }
            (Value::Tuple(values), Type::Array { elem, count }) => {
                ensure(values.len() == count)?;
                for val in values {
                    val.check(elem)?;
                }
            }
            _ => throw!(),
        }
    }
}

impl ValueExpr {
    fn check<Info: MemoryInfo>(self, locals: Map<LocalName, PlaceType>) -> Option<Type> {
        use ValueExpr::*;
        match self {
            Constant(value, ty) => {
                value.check(ty)?;
                ty
            }
            Load { source, destructive: _ } => {
                let ptype = source.check::<Info>(locals)?;
                ptype.ty
            }
            AddrOf { target, ptr_ty } => {
                let ptype = target.check::<Info>(locals)?;
                if let PtrType::Box { layout } | PtrType::Ref { layout, .. } = ptr_ty {
                    // Make sure the size fits and the alignment is weakened, not strengthened.
                    ensure(layout.size == ptype.size::<Info>())?;
                    ensure(layout.align <= ptype.align)?;
                }
                Type::Pointer(ptr_ty)
            }
            UnOp { operator, operand } => {
                let operand = operand.check::<Info>(locals)?;
                match operator {
                    UnOp::Int(_int_op, int_ty) => {
                        ensure(matches!(operand, Type::Int(_)))?;
                        Type::Int(int_ty)
                    }
                    UnOp::Ptr2Int => {
                        ensure(matches!(operand, Type::RawPtr))?;
                        Type::Int(IntType { signed: Unsigned, size: Info::PTR_SIZE })
                    }
                    UnOp::Int2Ptr => {
                        ensure(matches!(operand, Type::Int(IntType { signed: Unsigned, size: Info::PTR_SIZE })))?;
                        Type::RawPtr
                    }
                }
            }
            BinOp { operator, left, right } => {
                let left = left.check::<Info>(locals)?;
                let right = right.check::<Info>(locals)?;
                match operator {
                    BinOp::Int(_int_op, int_ty) => {
                        ensure(matches!(left, Type::Int(_)))?;
                        ensure(matches!(right, Type::Int(_)))?;
                        Type::Int(int_ty)
                    }
                    BinOp::PtrOffset { inbounds: _ } => {
                        ensure(matches!(left, Type::Ref { .. } | Type::RawPtr))?;
                        ensure(matches!(right, Type::Int(_)))?;
                        left
                    }
                }
            }
        }
    }
}

impl PlaceExpr {
    fn check<Info: MemoryInfo>(self, locals: Map<LocalName, PlaceType>) -> Option<PlaceType> {
        use PlaceExpr::*;
        match self {
            Local(name) => locals.get(name),
            Deref { operand, ptype } => {
                let ty = operand.check::<Info>(locals)?;
                ensure(matches!(ty, Type::Ref { .. } | Type::RawPtr))?;
                ptype
            }
            Field { root, field } => {
                let root = root.check::<Info>(locals)?;
                let (offset, field_ty) = match root.ty {
                    Type::Tuple { fields, .. } => fields.get(field)?,
                    Type::Union { fields, .. } => fields.get(field)?,
                    _ => throw!(),
                };
                PlaceType {
                    align: root.align.restrict_for_offset(offset),
                    ty: field_ty,
                }
            }
            Index { root, index } => {
                let root = root.check::<Info>(locals)?;
                let index = index.check::<Info>(locals)?;
                ensure(matches!(index, Type::Int(_)))?;
                let field_ty = match root.ty {
                    Type::Array { elem, .. } => elem,
                    _ => throw!(),
                };
                // We might be adding a multiple of `field_ty.size`, so we have to
                // lower the alignment compared to `root`. `restrict_for_offset`
                // is good for any multiple of that offset as well.
                PlaceType {
                    align: root.align.restrict_for_offset(field_ty.size::<Info>()),
                    ty: field_ty,
                }
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
    fn check<Info: MemoryInfo>(
        self,
        mut live_locals: Map<LocalName, PlaceType>,
        func: Function
    ) -> Option<Map<LocalName, PlaceType>> {
        use Statement::*;
        match self {
            Assign { destination, source } => {
                let left = destination.check::<Info>(live_locals)?;
                let right = source.check::<Info>(live_locals)?;
                ensure(left.ty == right)?;
                live_locals
            }
            Finalize { place } => {
                place.check::<Info>(live_locals)?;
                live_locals
            }
            StorageLive(local) => {
                // Look up the type in the function, and add it to the live locals.
                // Fail if it already is live.
                live_locals.try_insert(local, func.locals.get(local)?)?;
                live_locals
            }
            StorageDead(local) => {
                if func.ret.0 == local || func.args.iter().any(|(arg_name, _abi)| arg_name == local) {
                    // Trying to mark an argument or the return local as dead.
                    throw!();
                }
                live_locals.remove(local)?;
                live_locals
            }
        }
    }
}

impl Terminator {
    /// Returns the successor basic blocks that need to be checked next.
    fn check<Info: MemoryInfo>(
        self,
        live_locals: Map<LocalName, PlaceType>,
    ) -> Option<List<BbName>> {
        use Terminator::*;
        match self {
            Goto(block_name) => {
                list![block_name]
            }
            If { condition, then_block, else_block } => {
                let ty = condition.check::<Info>(live_locals)?;
                ensure(matches!(ty, Type::Bool))?;
                list![then_block, else_block]
            }
            Unreachable => {
                list![]
            }
            Call { callee: _, arguments, ret, next_block } => {
                // Argument and return expressions must all typecheck with some type.
                for (arg, _abi) in arguments {
                    arg.check::<Info>(live_locals)?;
                }
                let (ret_place, _ret_abi) = ret;
                ret_place.check::<Info>(live_locals)?;
                list![next_block]
            }
            Return => {
                list![]
            }
        }
    }
}

impl Function {
    fn check<Info: MemoryInfo>(self) -> Option<()> {
        // Construct initially live locals.
        // Also ensures that argument and return locals must exist.
        let mut start_live: Map<LocalName, PlaceType> = default();
        for (arg, _abi) in self.args {
            // Also ensures that no two arguments refer to the same local.
            start_live.try_insert(arg, self.locals.get(arg)?)?;
        }
        start_live.try_insert(self.ret.0, self.locals.get(self.ret.0)?)?;

        // Check the basic blocks. They can be cyclic, so we keep a worklist of
        // which blocks we still have to check. We also track the live locals
        // they start out with.
        let mut bb_live_at_entry: Map<BbName, Map<LocalName, PlaceType>> = default();
        bb_live_at_entry.insert(self.start, start_live);
        let mut todo = list![self.start];
        while let Some(block_name) = todo.pop_front() {
            let block = self.blocks.get(block_name)?;
            let mut live_locals = bb_live_at_entry[block_name];
            // Check this block, updating the live locals along the way.
            for statement in block.statements {
                live_locals = statement.check::<Info>(live_locals, self)?;
            }
            let successors = block.terminator.check::<Info>(live_locals)?;
            for block_name in successors {
                if let Some(precondition) = bb_live_at_entry.get(block_name) {
                    // A block we already visited (or already have in the worklist).
                    // Make sure the set of initially live locals is consistent!
                    ensure(precondition == live_locals)?;
                } else {
                    // A new block.
                    bb_live_at_entry.insert(block_name, live_locals);
                    todo.push_back(block_name);
                }
            }
        }

        // Ensure there are no dead blocks that we failed to reach.
        for block_name in self.blocks.keys() {
            ensure(bb_live_at_entry.contains(block_name))?;
        }
    }
}

impl Program {
    fn check<Info: MemoryInfo>(self) -> Option<()> {
        // Ensure the start function exists, and takes no arguments.
        let func = self.functions.get(self.start)?;
        if func.args.len() > 0 { return None; }
        // Check all the functions.
        for function in self.functions.values() {
            function.check::<Info>()?;
        }
    }
}
```
