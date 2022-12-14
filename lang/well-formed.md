# MiniRust well-formedness requirements

The various syntactic constructs of MiniRust (types, functions, ...) come with well-formedness requirements: certain invariants need to be satisfied for this to be considered a well-formed program.
The idea is that for well-formed programs, the `step` function will never panic.
Those requirements are defined in this file.

Note that `check_wf` functions for testing well-formedness return `Option<()>` rather than `bool` so that we can use `?`.
We use the following helper function to convert Boolean checks into this form.

```rust
fn ensure(b: bool) -> Option<()> {
    if !b { throw!(); }

    ret(())
}
```

## Well-formed layouts and types

```rust
impl IntType {
    fn check_wf(self) -> Option<()> {
        ensure(self.size.bytes().is_power_of_two())?;

        ret(())
    }
}

impl Layout {
    fn check_wf(self) -> Option<()> {
        // Nothing to check here.
        // In particular, we do *not* require that size is a multiple of align!
        // To represent e.g. the PlaceType of an `i32` at offset 0 in a
        // type that is `align(16)`, we have to be able to talk about types
        // with size 4 and alignment 16.

        ret(())
    }
}

impl PtrType {
    fn check_wf(self) -> Option<()> {
        match self {
            PtrType::Raw { pointee } | PtrType::Ref { pointee, mutbl: _ } | PtrType::Box { pointee } => {
                pointee.check_wf()?;
            }
        }

        ret(())
    }
}

impl Type {
    fn check_wf<M: Memory>(self) -> Option<()> {
        use Type::*;

        let size = self.size::<M>();
        ensure(M::valid_size(size))?;

        match self {
            Int(int_type) => {
                int_type.check_wf()?;
            }
            Bool => (),
            Ptr(ptr_type) => {
                ptr_type.check_wf()?;
            }
            Tuple { mut fields, size } => {
                // The fields must not overlap.
                // We check fields in the order of their (absolute) offsets.
                fields.sort_by_key(|(offset, _ty)| offset);
                let mut last_end = Size::ZERO;
                for (offset, ty) in fields {
                    // Recursively check the field type.
                    ty.check_wf::<M>()?;
                    // And ensure it fits after the one we previously checked.
                    ensure(offset >= last_end)?;
                    last_end = offset + ty.size::<M>();
                }
                // And they must all fit into the size.
                // The size is in turn checked to be valid for `M`, and hence all offsets are valid, too.
                ensure(size >= last_end)?;
            }
            Array { elem, count: _ } => {
                elem.check_wf::<M>()?;
            }
            Union { fields, size, chunks } => {
                // The fields may overlap, but they must all fit the size.
                for (offset, ty) in fields {
                    ty.check_wf::<M>()?;
                    ensure(size >= offset + ty.size::<M>())?;

                    // And it must fit into one of the chunks.
                    ensure(chunks.any(|(chunk_offset, chunk_size)| {
                        chunk_offset <= offset
                            && offset + ty.size::<M>() <= chunk_offset + chunk_size
                    }))?;
                }
                // The chunks must be sorted in their offsets and disjoint.
                // FIXME: should we relax this and allow arbitrary chunk order?
                let mut last_end = Size::ZERO;
                for (offset, size) in chunks {
                    ensure(offset >= last_end)?;
                    last_end = offset + size;
                }
                // And they must all fit into the size.
                ensure(size >= last_end)?;
            }
            Enum { variants, size, tag_encoding: _ } => {
                for variant in variants {
                    variant.check_wf::<M>()?;
                    ensure(size >= variant.size::<M>())?;
                }
            }
        }

        ret(())
    }
}

impl PlaceType {
    fn check_wf<M: Memory>(self) -> Option<()> {
        self.ty.check_wf::<M>()?;
        self.layout::<M>().check_wf()?;

        ret(())
    }
}
```

## Well-formed expressions

```rust
impl Constant {
    /// Check that the constant has the expected type.
    /// Assumes that `ty` has already been checked.
    fn check_wf(self, ty: Type) -> Option<()> {
        // For now, we only support integer and boolean literals, and arrays/tuples.
        // TODO: add more.
        match (self, ty) {
            (Constant::Int(i), Type::Int(int_type)) => {
                ensure(i.in_bounds(int_type.signed, int_type.size))?;
            }
            (Constant::Bool(_), Type::Bool) => (),
            (Constant::Tuple(constants), Type::Tuple { fields, size: _ }) => {
                ensure(constants.len() == fields.len())?;
                for (c, (_offset, ty)) in constants.zip(fields) {
                    c.check_wf(ty)?;
                }
            }
            (Constant::Tuple(constants), Type::Array { elem, count }) => {
                ensure(constants.len() == count)?;
                for c in constants {
                    c.check_wf(elem)?;
                }
            }
            (Constant::Variant { idx, data }, Type::Enum { variants, .. }) => {
                let ty = variants.get(idx)?;
                data.check_wf(ty)?;
            }
            _ => throw!(),
        }

        ret(())
    }
}

impl ValueExpr {
    fn check_wf<M: Memory>(self, locals: Map<LocalName, PlaceType>) -> Option<Type> {
        use ValueExpr::*;
        ret(match self {
            Constant(value, ty) => {
                value.check_wf(ty)?;
                ty
            }
            Load { source, destructive: _ } => {
                let ptype = source.check_wf::<M>(locals)?;
                ptype.ty
            }
            AddrOf { target, ptr_ty } => {
                let ptype = target.check_wf::<M>(locals)?;
                if let PtrType::Box { pointee } | PtrType::Ref { pointee, .. } = ptr_ty {
                    // Make sure the size fits and the alignment is weakened, not strengthened.
                    ensure(pointee.size == ptype.ty.size::<M>())?;
                    ensure(pointee.align <= ptype.align)?;
                }
                Type::Ptr(ptr_ty)
            }
            UnOp { operator, operand } => {
                use lang::UnOp::*;

                let operand = operand.check_wf::<M>(locals)?;
                match operator {
                    Int(_int_op, int_ty) => {
                        ensure(matches!(operand, Type::Int(_)))?;
                        Type::Int(int_ty)
                    }
                    Ptr2Int => {
                        ensure(matches!(operand, Type::Ptr(_)))?;
                        Type::Int(IntType { signed: Unsigned, size: M::PTR_SIZE })
                    }
                    Int2Ptr(ptr_ty) => {
                        ensure(operand == Type::Int(IntType { signed: Unsigned, size: M::PTR_SIZE }))?;
                        Type::Ptr(ptr_ty)
                    }
                }
            }
            BinOp { operator, left, right } => {
                use lang::BinOp::*;

                let left = left.check_wf::<M>(locals)?;
                let right = right.check_wf::<M>(locals)?;
                match operator {
                    Int(_int_op, int_ty) => {
                        ensure(matches!(left, Type::Int(_)))?;
                        ensure(matches!(right, Type::Int(_)))?;
                        Type::Int(int_ty)
                    }
                    PtrOffset { inbounds: _ } => {
                        ensure(matches!(left, Type::Ptr(_)))?;
                        ensure(matches!(right, Type::Int(_)))?;
                        left
                    }
                }
            }
        })
    }
}

impl PlaceExpr {
    fn check_wf<M: Memory>(self, locals: Map<LocalName, PlaceType>) -> Option<PlaceType> {
        use PlaceExpr::*;
        ret(match self {
            Local(name) => locals.get(name)?,
            Deref { operand, ptype } => {
                let ty = operand.check_wf::<M>(locals)?;
                ensure(matches!(ty, Type::Ptr(_)))?;
                ptype
            }
            Field { root, field } => {
                let root = root.check_wf::<M>(locals)?;
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
                let root = root.check_wf::<M>(locals)?;
                let index = index.check_wf::<M>(locals)?;
                ensure(matches!(index, Type::Int(_)))?;
                let field_ty = match root.ty {
                    Type::Array { elem, .. } => elem,
                    _ => throw!(),
                };
                // We might be adding a multiple of `field_ty.size`, so we have to
                // lower the alignment compared to `root`. `restrict_for_offset`
                // is good for any multiple of that offset as well.
                PlaceType {
                    align: root.align.restrict_for_offset(field_ty.size::<M>()),
                    ty: field_ty,
                }
            }
        })
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
    fn check_wf<M: Memory>(
        self,
        mut live_locals: Map<LocalName, PlaceType>,
        func: Function
    ) -> Option<Map<LocalName, PlaceType>> {
        use Statement::*;
        ret(match self {
            Assign { destination, source } => {
                let left = destination.check_wf::<M>(live_locals)?;
                let right = source.check_wf::<M>(live_locals)?;
                ensure(left.ty == right)?;
                live_locals
            }
            Finalize { place, fn_entry: _ } => {
                place.check_wf::<M>(live_locals)?;
                live_locals
            }
            StorageLive(local) => {
                // Look up the type in the function, and add it to the live locals.
                // Fail if it already is live.
                live_locals.try_insert(local, func.locals.get(local)?).ok()?;
                live_locals
            }
            StorageDead(local) => {
                if func.ret.is_some_and(|(l, _)| l == local) || func.args.any(|(arg_name, _abi)| arg_name == local) {
                    // Trying to mark an argument or the return local as dead.
                    throw!();
                }
                live_locals.remove(local)?;
                live_locals
            }
        })
    }
}

impl Terminator {
    /// Returns the successor basic blocks that need to be checked next.
    fn check_wf<M: Memory>(
        self,
        live_locals: Map<LocalName, PlaceType>,
    ) -> Option<List<BbName>> {
        use Terminator::*;
        ret(match self {
            Goto(block_name) => {
                list![block_name]
            }
            If { condition, then_block, else_block } => {
                let ty = condition.check_wf::<M>(live_locals)?;
                ensure(matches!(ty, Type::Bool))?;
                list![then_block, else_block]
            }
            Unreachable => {
                list![]
            }
            Call { callee: _, arguments, ret, next_block } => {
                // Argument and return expressions must all typecheck with some type.
                for (arg, _abi) in arguments {
                    arg.check_wf::<M>(live_locals)?;
                }

                if let Some((ret_place, _ret_abi)) = ret {
                    ret_place.check_wf::<M>(live_locals)?;
                }

                match next_block {
                    Some(b) => list![b],
                    None => list![],
                }
            }
            CallIntrinsic { intrinsic: _, arguments, ret, next_block } => {
                // Argument and return expressions must all typecheck with some type.
                for arg in arguments {
                    arg.check_wf::<M>(live_locals)?;
                }

                if let Some(ret_place) = ret {
                    ret_place.check_wf::<M>(live_locals)?;
                }

                match next_block {
                    Some(b) => list![b],
                    None => list![],
                }
            }
            Return => {
                list![]
            }
        })
    }
}

impl Function {
    fn check_wf<M: Memory>(self) -> Option<()> {
        // Ensure all locals have a valid type.
        for pty in self.locals.values() {
            pty.check_wf::<M>()?;
        }

        // Construct initially live locals.
        // Also ensures that argument and return locals must exist.
        let mut start_live: Map<LocalName, PlaceType> = Map::new();
        for (arg, _abi) in self.args {
            // Also ensures that no two arguments refer to the same local.
            start_live.try_insert(arg, self.locals.get(arg)?).ok()?;
        }
        if let Some((ret, _abi)) = self.ret {
            start_live.try_insert(ret, self.locals.get(ret)?).ok()?;
        }

        // Check the basic blocks. They can be cyclic, so we keep a worklist of
        // which blocks we still have to check. We also track the live locals
        // they start out with.
        let mut bb_live_at_entry: Map<BbName, Map<LocalName, PlaceType>> = Map::new();
        bb_live_at_entry.insert(self.start, start_live);
        let mut todo = list![self.start];
        while let Some(block_name) = todo.pop_front() {
            let block = self.blocks.get(block_name)?;
            let mut live_locals = bb_live_at_entry[block_name];
            // Check this block, updating the live locals along the way.
            for statement in block.statements {
                live_locals = statement.check_wf::<M>(live_locals, self)?;
            }
            let successors = block.terminator.check_wf::<M>(live_locals)?;
            for block_name in successors {
                if let Some(precondition) = bb_live_at_entry.get(block_name) {
                    // A block we already visited (or already have in the worklist).
                    // Make sure the set of initially live locals is consistent!
                    ensure(precondition == live_locals)?;
                } else {
                    // A new block.
                    bb_live_at_entry.insert(block_name, live_locals);
                    todo.push(block_name);
                }
            }
        }

        // Ensure there are no dead blocks that we failed to reach.
        for block_name in self.blocks.keys() {
            ensure(bb_live_at_entry.contains_key(block_name))?;
        }

        ret(())
    }
}

impl Program {
    fn check_wf<M: Memory>(self) -> Option<()> {
        // Ensure the start function exists, and takes no arguments.
        let func = self.functions.get(self.start)?;
        ensure(func.args.is_empty())?;
        ensure(func.ret.is_none())?;
        // Check all the functions.
        for function in self.functions.values() {
            function.check_wf::<M>()?;
        }

        ret(())
    }
}
```
