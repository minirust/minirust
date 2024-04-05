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
        // In particular, this checks that the size is at least one byte.
        ensure(self.size.bytes().is_power_of_two())?;

        ret(())
    }
}

impl Layout {
    fn check_wf<T: Target>(self) -> Option<()> {
        // We do *not* require that size is a multiple of align!
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

        // Ensure that the size is valid and a multiple of the alignment.
        let size = self.size::<T>();
        ensure(T::valid_size(size))?;
        let align = self.align::<T>();
        ensure(size.bytes() % align.bytes() == 0)?;

        match self {
            Int(int_type) => {
                int_type.check_wf()?;
            }
            Bool => (),
            Ptr(ptr_type) => {
                ptr_type.check_wf::<T>()?;
            }
            Tuple { mut fields, size, align: _ } => {
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
            Union { fields, size, chunks, align: _ } => {
                // The fields may overlap, but they must all fit the size.
                for (offset, ty) in fields {
                    ty.check_wf::<T>()?;
                    ensure(size >= offset + ty.size::<T>())?;
                    // This field may overlap with gaps between the chunks. That's perfectly normal
                    // when there is padding inside the field.
                    // FIXME: should we check that all the non-padding bytes of the field are in some chunk?
                    // But then we'd have to add a definition of "used (non-padding) bytes" in the spec, and then
                    // we may as well remove 'chunks' entirely and just compute the set of used bytes for
                    // encoding/decoding...
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
            Enum { variants, size, discriminator, discriminant_ty, .. } => {
                // All the variants need to be well-formed and be the size of the enum so
                // we don't have to handle different sizes in the memory representation.
                // Also their alignment may not be larger than the total enum alignment and
                // all the values written by the tagger must fit into the variant.
                for (discriminant, variant) in variants {
                    ensure(discriminant_ty.can_represent(discriminant))?;

                    variant.ty.check_wf::<T>()?;
                    ensure(size == variant.ty.size::<T>())?;
                    ensure(variant.ty.align::<T>().bytes() <= align.bytes())?;
                    ensure(variant.tagger.iter().all(|(offset, (value_type, value))|
                        value_type.check_wf().is_some() &&
                        value_type.can_represent(value) &&
                        offset + value_type.size <= size
                    ))?;
                    // FIXME: check that the values written by the tagger do not overlap.
                }

                // check that all variants reached by the discriminator are valid,
                // that it never performs out-of-bounds accesses and all discriminant values
                // can be represented by the discriminant type.
                discriminator.check_wf::<T>(size, variants)?;
            }
        }

        ret(())
    }
}

impl Discriminator {
    fn check_wf<T: Target>(self, size: Size, variants: Map<Int, Variant>) -> Option<()> {
        match self {
            Discriminator::Known(discriminant) => ensure(variants.get(discriminant).is_some()),
            Discriminator::Invalid => ret(()),
            Discriminator::Branch { offset, value_type, fallback, children } => {
                // Ensure that the value we branch on is stored in bounds and that all children all valid.
                value_type.check_wf()?;
                ensure(offset + value_type.size <= size)?;
                fallback.check_wf::<T>(size, variants)?;
                for (idx, ((start, end), discriminator)) in children.into_iter().enumerate() {
                    ensure(value_type.can_represent(start))?;
                    // Since the end is exclusive we only need to represent the number before the end.
                    ensure(value_type.can_represent(end - Int::ONE))?;
                    ensure(start < end)?;
                    // Ensure that the ranges don't overlap.
                    ensure(children.keys().enumerate().all(|(other_idx, (other_start, other_end))| other_end <= start || other_start >= end || idx == other_idx))?;
                    discriminator.check_wf::<T>(size, variants)?;
                }
                ret(())
            }
        }
    }
}
```

## Well-formed expressions

```rust
impl Constant {
    /// Check that the constant has the expected type.
    /// Assumes that `ty` has already been checked.
    fn check_wf<T: Target>(self, ty: Type, prog: Program) -> Option<()> {
        // For now, we only support integer and boolean literals and pointers.
        // TODO: add more.
        match (self, ty) {
            (Constant::Int(i), Type::Int(int_type)) => {
                ensure(int_type.can_represent(i))?;
            }
            (Constant::Bool(_), Type::Bool) => (),
            (Constant::GlobalPointer(relocation), Type::Ptr(_)) => {
                relocation.check_wf(prog.globals)?;
            }
            (Constant::FnPointer(fn_name), Type::Ptr(_)) => {
                ensure(prog.functions.contains_key(fn_name))?;
            }
            (Constant::PointerWithoutProvenance(addr), Type::Ptr(_)) => {
                ensure(addr.in_bounds(Signedness::Unsigned, T::PTR_SIZE))?;
            }
            _ => throw!(),
        }

        ret(())
    }
}

impl ValueExpr {
    #[allow(unused_braces)]
    fn check_wf<T: Target>(self, locals: Map<LocalName, Type>, prog: Program) -> Option<Type> {
        use ValueExpr::*;
        ret(match self {
            Constant(value, ty) => {
                ty.check_wf::<T>()?;
                value.check_wf::<T>(ty, prog)?;
                ty
            }
            Tuple(exprs, t) => {
                t.check_wf::<T>()?;

                match t {
                    Type::Tuple { fields, .. } => {
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
            Variant { discriminant, data, enum_ty } => {
                let Type::Enum { variants, .. } = enum_ty else { throw!() };
                enum_ty.check_wf::<T>()?;
                let ty = variants.get(discriminant)?.ty;

                let checked = data.check_wf::<T>(locals, prog)?;
                ensure(checked == ty);
                enum_ty
            }
            GetDiscriminant { place } => {
                let Some(Type::Enum { discriminant_ty, .. }) = place.check_wf::<T>(locals, prog) else {
                    throw!();
                };
                Type::Int(discriminant_ty)
            }
            Load { source } => {
                source.check_wf::<T>(locals, prog)?
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
                    Int(_int_op) => {
                        let Type::Int(int_ty) = operand else {
                            throw!();
                        };
                        Type::Int(int_ty)
                    }
                    Bool(_bool_op) => {
                        ensure(matches!(operand, Type::Bool))?;
                        Type::Bool
                    }
                    Cast(cast_op) => {
                        use lang::CastOp::*;
                        match cast_op {
                            IntToInt(int_ty) => {
                                ensure(matches!(operand, Type::Int(_)))?;
                                Type::Int(int_ty)
                            }
                            BoolToInt(int_ty) => {
                                ensure(matches!(operand, Type::Bool))?;
                                Type::Int(int_ty)
                            }
                            Transmute(new_ty) => {
                                new_ty
                            }
                        }
                    }
                }
            }
            BinOp { operator, left, right } => {
                use lang::BinOp::*;

                let left = left.check_wf::<T>(locals, prog)?;
                let right = right.check_wf::<T>(locals, prog)?;
                match operator {
                    Int(_int_op) => {
                        let Type::Int(int_ty) = left else {
                            throw!();
                        };
                        ensure(right == Type::Int(int_ty))?;
                        Type::Int(int_ty)
                    }
                    IntRel(_int_rel) => {
                        let Type::Int(int_ty) = left else {
                            throw!();
                        };
                        ensure(right == Type::Int(int_ty))?;
                        Type::Bool
                    }
                    PtrOffset { inbounds: _ } => {
                        ensure(matches!(left, Type::Ptr(_)))?;
                        ensure(matches!(right, Type::Int(_)))?;
                        left
                    }
                    Bool(_bool_op) => {
                        ensure(matches!(left, Type::Bool))?;
                        ensure(matches!(right, Type::Bool))?;
                        Type::Bool
                    }
                }
            }
        })
    }
}

impl PlaceExpr {
    fn check_wf<T: Target>(self, locals: Map<LocalName, Type>, prog: Program) -> Option<Type> {
        use PlaceExpr::*;
        ret(match self {
            Local(name) => locals.get(name)?,
            Deref { operand, ty } => {
                let op_ty = operand.check_wf::<T>(locals, prog)?;
                ensure(matches!(op_ty, Type::Ptr(_)))?;
                // No check of how the alignment changes here -- that is purely a runtime constraint.
                ty
            }
            Field { root, field } => {
                let root = root.check_wf::<T>(locals, prog)?;
                let (_offset, field_ty) = match root {
                    Type::Tuple { fields, .. } => fields.get(field)?,
                    Type::Union { fields, .. } => fields.get(field)?,
                    _ => throw!(),
                };
                field_ty
            }
            Index { root, index } => {
                let root = root.check_wf::<T>(locals, prog)?;
                let index = index.check_wf::<T>(locals, prog)?;
                ensure(matches!(index, Type::Int(_)))?;
                match root {
                    Type::Array { elem, .. } => elem,
                    _ => throw!(),
                }
            }
            Downcast { root, discriminant } => {
                let root = root.check_wf::<T>(locals, prog)?;
                match root {
                    // A valid downcast points to an existing variant.
                    Type::Enum { variants, .. } => variants.get(discriminant)?.ty,
                    _ => throw!(),
                }
            }
        })
    }
}

impl ArgumentExpr {
    fn check_wf<T: Target>(self, locals: Map<LocalName, Type>, prog: Program) -> Option<Type> {
        ret(match self {
            ArgumentExpr::ByValue(value) => value.check_wf::<T>(locals, prog)?,
            ArgumentExpr::InPlace(place) => place.check_wf::<T>(locals, prog)?
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
        mut live_locals: Map<LocalName, Type>,
        func: Function,
        prog: Program,
    ) -> Option<Map<LocalName, Type>> {
        use Statement::*;
        ret(match self {
            Assign { destination, source } => {
                let left = destination.check_wf::<T>(live_locals, prog)?;
                let right = source.check_wf::<T>(live_locals, prog)?;
                ensure(left == right)?;
                live_locals
            }
            SetDiscriminant { destination, value } => {
                let Type::Enum { variants, .. } = destination.check_wf::<T>(live_locals, prog)? else {
                    throw!();
                };
                // We don't ensure that we can actually represent the discriminant.
                // The well-formedness checks for the type just ensure that every discriminant
                // reached by the discriminator is valid, however there we don't require that every
                // variant is represented. Setting such an unrepresented discriminant would probably
                // result in an invalid value as either the discriminator returns
                // `Discriminator::Invalid` or another variant.
                // This is fine as SetDiscriminant does not guarantee that the enum is a valid value.
                variants.get(value)?;
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
                if local == func.ret || func.args.any(|arg_name| local == arg_name) {
                    // Trying to mark an argument or the return local as dead.
                    throw!();
                }
                live_locals.remove(local)?;
                live_locals
            }
        })
    }
}

/// Predicate to indicate if integer bin-op can be used for atomic fetch operations.
/// Needed for atomic fetch operations.
/// 
/// We limit the binops that are allowed to be atomic based on current LLVM and Rust API exposures.
fn is_atomic_binop(op: BinOpInt) -> bool {
    use BinOpInt as B;
    match op {
        B::Add | B::Sub => true,
        _ => false
    }
}

impl Terminator {
    /// Returns the successor basic blocks that need to be checked next.
    fn check_wf<T: Target>(
        self,
        live_locals: Map<LocalName, Type>,
        prog: Program,
    ) -> Option<List<BbName>> {
        use Terminator::*;
        ret(match self {
            Goto(block_name) => {
                list![block_name]
            }
            Switch { value, cases, fallback } => {
                let ty = value.check_wf::<T>(live_locals, prog)?;
                let Type::Int(switch_ty) = ty else {
                    // We only switch on integers.
                    // This is in contrast to Rust MIR where switch can work on `char`s and booleans as well.
                    // However since those are trivial casts we chose to only accept integers.
                    throw!()
                };

                // ensures that all cases are valid and therefore can be reached from this block.
                let mut next_blocks = List::new();
                for (case, block) in cases.iter() {
                    ensure(switch_ty.can_represent(case))?;
                    next_blocks.push(block);
                }

                // we can also reach the fallback block.
                next_blocks.push(fallback);
                next_blocks
            }
            Unreachable => {
                list![]
            }
            Call { callee, arguments, ret, next_block } => {
                let ty = callee.check_wf::<T>(live_locals, prog)?;
                ensure(matches!(ty, Type::Ptr(PtrType::FnPtr(_))))?;

                // Return and argument expressions must all typecheck with some type.
                ret.check_wf::<T>(live_locals, prog)?;
                for arg in arguments {
                    arg.check_wf::<T>(live_locals, prog)?;
                }

                match next_block {
                    Some(b) => list![b],
                    None => list![],
                }
            }
            Intrinsic { intrinsic, arguments, ret, next_block } => {
                // Return and argument expressions must all typecheck with some type.
                ret.check_wf::<T>(live_locals, prog)?;
                for arg in arguments {
                    arg.check_wf::<T>(live_locals, prog)?;
                }

                // Currently only AtomicFetchAndOp has special well-formedness requirements.
                match intrinsic {
                    IntrinsicOp::AtomicFetchAndOp(op) => {
                        if !is_atomic_binop(op) {
                            throw!();
                        }
                    }
                    _ => {}
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
        for ty in self.locals.values() {
            ty.check_wf::<T>()?;
        }

        // Construct initially live locals.
        // Also ensures that return and argument locals must exist.
        let mut start_live: Map<LocalName, Type> = Map::new();
        start_live.try_insert(self.ret, self.locals.get(self.ret)?).ok()?;
        for arg in self.args {
            // Also ensures that no two arguments refer to the same local.
            start_live.try_insert(arg, self.locals.get(arg)?).ok()?;
        }

        // Check the basic blocks. They can be cyclic, so we keep a worklist of
        // which blocks we still have to check. We also track the live locals
        // they start out with.
        let mut bb_live_at_entry: Map<BbName, Map<LocalName, Type>> = Map::new();
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
        // Ensure the start function exists, has the right ABI, takes no arguments, and returns a 1-ZST.
        let func = self.functions.get(self.start)?;
        ensure(func.calling_convention == CallingConvention::C);
        let ret_layout = func.locals.get(func.ret)?.layout::<T>();
        ensure(ret_layout.size == Size::ZERO && ret_layout.align == Align::ONE);
        ensure(func.args.is_empty())?;

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
                ensure(ity.can_represent(i))?;
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
            (Value::Variant { discriminant, data }, Type::Enum { variants, .. }) => {
                let variant = variants.get(discriminant)?.ty;
                data.check_wf(variant)?;
            }
            _ => throw!()
        }

        ret(())
    }
}
```
