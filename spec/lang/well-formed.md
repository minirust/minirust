# MiniRust well-formedness requirements

The various syntactic constructs of MiniRust (types, functions, ...) come with well-formedness requirements: certain invariants need to be satisfied for this to be considered a well-formed program.
The idea is that for well-formed programs, the `step` function will never panic.
Those requirements are defined in this file.

We also define the idea of a "value being well-formed at a type".
`decode` will only ever return well-formed values, and `encode` will never panic on a well-formed value.

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
    fn check_wf<T: Target>(self) -> Option<()> {
        // We do *not* require that size is a multiple of align!
        // To represent e.g. the PlaceType of an `i32` at offset 0 in a
        // type that is `align(16)`, we have to be able to talk about types
        // with size 4 and alignment 16.
        ensure(T::valid_size(self.size))?;

        ret(())
    }
}

impl PtrType {
    fn check_wf<T: Target>(self) -> Option<()> {
        match self {
            PtrType::Ref { pointee, mutbl: _ } | PtrType::Box { pointee } => {
                pointee.check_wf::<T>()?;
            }
            PtrType::Raw | PtrType::FnPtr(_) => ()
        }

        ret(())
    }
}

impl Type {
    fn check_wf<T: Target>(self) -> Option<()> {
        use Type::*;

        let size = self.size::<T>();
        ensure(T::valid_size(size))?;

        match self {
            Int(int_type) => {
                int_type.check_wf()?;
            }
            Bool => (),
            Ptr(ptr_type) => {
                ptr_type.check_wf::<T>()?;
            }
            Tuple { mut fields, size } => {
                // The fields must not overlap.
                // We check fields in the order of their (absolute) offsets.
                fields.sort_by_key(|(offset, _ty)| offset);
                let mut last_end = Size::ZERO;
                for (offset, ty) in fields {
                    // Recursively check the field type.
                    ty.check_wf::<T>()?;
                    // And ensure it fits after the one we previously checked.
                    ensure(offset >= last_end)?;
                    last_end = offset + ty.size::<T>();
                }
                // And they must all fit into the size.
                // The size is in turn checked to be valid for `M`, and hence all offsets are valid, too.
                ensure(size >= last_end)?;
            }
            Array { elem, count } => {
                ensure(count >= 0)?;
                elem.check_wf::<T>()?;
            }
            Union { fields, size, chunks } => {
                // The fields may overlap, but they must all fit the size.
                for (offset, ty) in fields {
                    ty.check_wf::<T>()?;
                    ensure(size >= offset + ty.size::<T>())?;

                    // And it must fit into one of the chunks.
                    ensure(chunks.any(|(chunk_offset, chunk_size)| {
                        chunk_offset <= offset
                            && offset + ty.size::<T>() <= chunk_offset + chunk_size
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
                    variant.check_wf::<T>()?;
                    ensure(size >= variant.size::<T>())?;
                }
            }
        }

        ret(())
    }
}

impl PlaceType {
    fn check_wf<T: Target>(self) -> Option<()> {
        let PlaceType { ty, align: _ } = self;
        ty.check_wf::<T>()?;

        ret(())
    }
}
```

## Well-formed expressions

```rust
impl Constant {
    /// Check that the constant has the expected type.
    /// Assumes that `ty` has already been checked.
    fn check_wf(self, ty: Type, prog: Program) -> Option<()> {
        // For now, we only support integer and boolean literals and pointers.
        // TODO: add more.
        match (self, ty) {
            (Constant::Int(i), Type::Int(int_type)) => {
                ensure(i.in_bounds(int_type.signed, int_type.size))?;
            }
            (Constant::Bool(_), Type::Bool) => (),
            (Constant::Variant { idx, data }, Type::Enum { variants, .. }) => {
                let ty = variants.get(idx)?;
                data.check_wf(ty, prog)?;
            }
            (Constant::GlobalPointer(relocation), Type::Ptr(_)) => {
                relocation.check_wf(prog.globals)?;
            }
            (Constant::FnPointer(fn_name), Type::Ptr(_)) => {
                ensure(prog.functions.contains_key(fn_name))?;
            }
            (Constant::Null, Type::Ptr(_)) => {}
            _ => throw!(),
        }

        ret(())
    }
}

impl ValueExpr {
    fn check_wf<T: Target>(self, locals: Map<LocalName, PlaceType>, prog: Program) -> Option<Type> {
        use ValueExpr::*;
        ret(match self {
            Constant(value, ty) => {
                ty.check_wf::<T>()?;

                value.check_wf(ty, prog)?;
                ty
            }
            Tuple(exprs, t) => {
                t.check_wf::<T>()?;

                match t {
                    Type::Tuple { fields, size: _ } => {
                        ensure(exprs.len() == fields.len())?;
                        for (e, (_offset, ty)) in exprs.zip(fields) {
                            let checked = e.check_wf::<T>(locals, prog)?;
                            ensure(checked == ty)?;
                        }
                    },
                    Type::Array { elem, count } => {
                        ensure(exprs.len() == count)?;
                        for e in exprs {
                            let checked = e.check_wf::<T>(locals, prog)?;
                            ensure(checked == elem)?;
                        }
                    },
                    _ => throw!(),
                }

                t
            }
            Union { field, expr, union_ty } => {
                union_ty.check_wf::<T>()?;

                let Type::Union { fields, .. } = union_ty else { throw!() };

                ensure(field < fields.len())?;
                let (_offset, ty) = fields[field];

                let checked = expr.check_wf::<T>(locals, prog)?;
                ensure(checked == ty)?;

                union_ty
            }
            Load { source } => {
                let ptype = source.check_wf::<T>(locals, prog)?;
                ptype.ty
            }
            AddrOf { target, ptr_ty } => {
                target.check_wf::<T>(locals, prog)?;
                // No check of how the alignment changes here -- that is purely a runtime constraint.
                Type::Ptr(ptr_ty)
            }
            UnOp { operator, operand } => {
                use lang::UnOp::*;

                let operand = operand.check_wf::<T>(locals, prog)?;
                match operator {
                    Int(_int_op, int_ty) => {
                        ensure(matches!(operand, Type::Int(_)))?;
                        Type::Int(int_ty)
                    }
                    PtrCast(ptr_ty) => {
                        ensure(matches!(operand, Type::Ptr(_)))?;
                        Type::Ptr(ptr_ty)
                    }
                    PtrAddr => {
                        ensure(matches!(operand, Type::Ptr(_)))?;
                        Type::Int(IntType { signed: Unsigned, size: T::PTR_SIZE })
                    }
                    PtrFromExposed(ptr_ty) => {
                        ensure(operand == Type::Int(IntType { signed: Unsigned, size: T::PTR_SIZE }))?;
                        Type::Ptr(ptr_ty)
                    }
                }
            }
            BinOp { operator, left, right } => {
                use lang::BinOp::*;

                let left = left.check_wf::<T>(locals, prog)?;
                let right = right.check_wf::<T>(locals, prog)?;
                match operator {
                    Int(_int_op, int_ty) => {
                        ensure(matches!(left, Type::Int(_)))?;
                        ensure(matches!(right, Type::Int(_)))?;
                        Type::Int(int_ty)
                    }
                    IntRel(_int_rel) => {
                        ensure(matches!(left, Type::Int(_)))?;
                        ensure(matches!(right, Type::Int(_)))?;
                        Type::Bool
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
    fn check_wf<T: Target>(self, locals: Map<LocalName, PlaceType>, prog: Program) -> Option<PlaceType> {
        use PlaceExpr::*;
        ret(match self {
            Local(name) => locals.get(name)?,
            Deref { operand, ptype } => {
                let ty = operand.check_wf::<T>(locals, prog)?;
                ensure(matches!(ty, Type::Ptr(_)))?;
                // No check of how the alignment changes here -- that is purely a runtime constraint.
                ptype
            }
            Field { root, field } => {
                let root = root.check_wf::<T>(locals, prog)?;
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
                let root = root.check_wf::<T>(locals, prog)?;
                let index = index.check_wf::<T>(locals, prog)?;
                ensure(matches!(index, Type::Int(_)))?;
                let field_ty = match root.ty {
                    Type::Array { elem, .. } => elem,
                    _ => throw!(),
                };
                // We might be adding a multiple of `field_ty.size`, so we have to
                // lower the alignment compared to `root`. `restrict_for_offset`
                // is good for any multiple of that offset as well.
                PlaceType {
                    align: root.align.restrict_for_offset(field_ty.size::<T>()),
                    ty: field_ty,
                }
            }
        })
    }
}

impl ArgumentExpr {
    fn check_wf<T: Target>(self, locals: Map<LocalName, PlaceType>, prog: Program) -> Option<Type> {
        ret(match self {
            ArgumentExpr::ByValue(value, _align) => value.check_wf::<T>(locals, prog)?,
            ArgumentExpr::InPlace(place) => place.check_wf::<T>(locals, prog)?.ty
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
    fn check_wf<T: Target>(
        self,
        mut live_locals: Map<LocalName, PlaceType>,
        func: Function,
        prog: Program,
    ) -> Option<Map<LocalName, PlaceType>> {
        use Statement::*;
        ret(match self {
            Assign { destination, source } => {
                let left = destination.check_wf::<T>(live_locals, prog)?;
                let right = source.check_wf::<T>(live_locals, prog)?;
                ensure(left.ty == right)?;
                live_locals
            }
            Expose { value } => {
                let v = value.check_wf::<T>(live_locals, prog)?;
                ensure(matches!(v, Type::Ptr(_)));
                live_locals
            }
            Validate { place, fn_entry: _ } => {
                place.check_wf::<T>(live_locals, prog)?;
                live_locals
            }
            Deinit { place } => {
                place.check_wf::<T>(live_locals, prog)?;
                live_locals
            }
            StorageLive(local) => {
                // Look up the type in the function, and add it to the live locals.
                // Fail if it already is live.
                live_locals.try_insert(local, func.locals.get(local)?).ok()?;
                live_locals
            }
            StorageDead(local) => {
                if func.ret.is_some_and(|l| l == local) || func.args.any(|arg_name| arg_name == local) {
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
    fn check_wf<T: Target>(
        self,
        live_locals: Map<LocalName, PlaceType>,
        prog: Program,
    ) -> Option<List<BbName>> {
        use Terminator::*;
        ret(match self {
            Goto(block_name) => {
                list![block_name]
            }
            If { condition, then_block, else_block } => {
                let ty = condition.check_wf::<T>(live_locals, prog)?;
                ensure(matches!(ty, Type::Bool))?;
                list![then_block, else_block]
            }
            Unreachable => {
                list![]
            }
            Call { callee, arguments, ret, next_block } => {
                let ty = callee.check_wf::<T>(live_locals, prog)?;
                ensure(matches!(ty, Type::Ptr(PtrType::FnPtr(_))))?;

                // Argument and return expressions must all typecheck with some type.
                for arg in arguments {
                    arg.check_wf::<T>(live_locals, prog)?;
                }

                if let Some(ret_place) = ret {
                    ret_place.check_wf::<T>(live_locals, prog)?;
                }

                match next_block {
                    Some(b) => list![b],
                    None => list![],
                }
            }
            CallIntrinsic { intrinsic: _, arguments, ret, next_block } => {
                // Argument and return expressions must all typecheck with some type.
                for arg in arguments {
                    arg.check_wf::<T>(live_locals, prog)?;
                }

                if let Some(ret_place) = ret {
                    ret_place.check_wf::<T>(live_locals, prog)?;
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
    fn check_wf<T: Target>(self, prog: Program) -> Option<()> {
        // Ensure all locals have a valid type.
        for pty in self.locals.values() {
            pty.check_wf::<T>()?;
        }

        // Construct initially live locals.
        // Also ensures that argument and return locals must exist.
        let mut start_live: Map<LocalName, PlaceType> = Map::new();
        for arg in self.args {
            // Also ensures that no two arguments refer to the same local.
            start_live.try_insert(arg, self.locals.get(arg)?).ok()?;
        }
        if let Some(ret) = self.ret {
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
                live_locals = statement.check_wf::<T>(live_locals, self, prog)?;
            }
            let successors = block.terminator.check_wf::<T>(live_locals, prog)?;
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

impl Relocation {
    // Checks whether the relocation is within bounds.
    fn check_wf(self, globals: Map<GlobalName, Global>) -> Option<()> {
        // The global we are pointing to needs to exist.
        let global = globals.get(self.name)?;
        let size = Size::from_bytes(global.bytes.len()).unwrap();

        // And the offset needs to be in-bounds of its size.
        ensure(self.offset <= size)?;

        ret(())
    }
}

impl Program {
    fn check_wf<T: Target>(self) -> Option<()> {
        // Ensure the start function exists, has the right ABI, takes no arguments, and does not return.
        let func = self.functions.get(self.start)?;
        ensure(func.calling_convention == CallingConvention::C);
        ensure(func.args.is_empty())?;
        ensure(func.ret.is_none())?;
        // Check all the functions.
        for function in self.functions.values() {
            function.check_wf::<T>(self)?;
        }

        // Check globals.
        for (_name, global) in self.globals {
            let size = Size::from_bytes(global.bytes.len()).unwrap();
            for (offset, relocation) in global.relocations {
                // A relocation fills `PTR_SIZE` many bytes starting at the offset, those need to fit into the size.
                ensure(offset + T::PTR_SIZE <= size)?;

                relocation.check_wf(self.globals)?;
            }
        }

        ret(())
    }
}
```

## Well-formed values

```rust
impl<M: Memory> Value<M> {
    /// We assume `ty` is itself well-formed.
    fn check_wf(self, ty: Type) -> Option<()> {
        match (self, ty) {
            (Value::Int(i), Type::Int(ity)) => {
                ensure(i.in_bounds(ity.signed, ity.size))?;
            }
            (Value::Bool(_), Type::Bool) => {},
            (Value::Ptr(ptr), Type::Ptr(ptr_ty)) => {
                ensure(ptr_ty.addr_valid(ptr.addr))?;
                ensure(ptr.addr.in_bounds(Unsigned, M::T::PTR_SIZE))?;
            }
            (Value::Tuple(vals), Type::Tuple { fields, .. }) => {
                ensure(vals.len() == fields.len())?;
                for (val, (_, ty)) in vals.zip(fields) {
                    val.check_wf(ty)?;
                }
            }
            (Value::Tuple(vals), Type::Array { elem, count }) => {
                ensure(vals.len() == count)?;
                for val in vals {
                    val.check_wf(elem)?;
                }
            }
            (Value::Union(chunk_data), Type::Union { chunks, .. }) => {
                ensure(chunk_data.len() == chunks.len())?;
                for (data, (_, size)) in chunk_data.zip(chunks) {
                    ensure(data.len() == size.bytes())?;
                }
            }
            (Value::Variant { idx, data }, Type::Enum { variants, .. }) => {
                ensure(idx < variants.len())?;
                data.check_wf(variants[idx])?;
            }
            _ => throw!()
        }

        ret(())
    }
}
```
