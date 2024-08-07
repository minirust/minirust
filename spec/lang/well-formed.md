# MiniRust well-formedness requirements

The various syntactic constructs of MiniRust (types, functions, ...) come with well-formedness requirements: certain invariants need to be satisfied for this to be considered a well-formed program.
The idea is that for well-formed programs, the `step` function will never panic.
Those requirements are defined in this file.

We also define the idea of a "value being well-formed at a type".
`decode` will only ever return well-formed values, and `encode` will never panic on a well-formed value.

Note that `check_wf` functions for testing well-formedness return `Result<()>` to pass information in case an error occured.

We use the following helper function to convert Boolean checks into this form.

```rust
fn ensure_wf(b: bool, msg: &str) -> Result<()> {
    if !b { throw_ill_formed!("{}", msg); }
    ret(())
}
```

## Well-formed layouts and types

```rust
impl IntType {
    fn check_wf(self) -> Result<()> {
        // In particular, this checks that the size is at least one byte.
        ensure_wf(self.size.bytes().is_power_of_two(), "IntType: size is not power of two")
    }
}

impl PointeeInfo {
    fn check_wf<T: Target>(self) -> Result<()> {
        // We do *not* require that size is a multiple of align!
        ensure_wf(T::valid_size(self.size), "Layout: size not valid")
    }
}

impl PtrType {
    fn check_wf<T: Target>(self) -> Result<()> {
        match self {
            PtrType::Ref { pointee, mutbl: _ } | PtrType::Box { pointee } => {
                pointee.check_wf::<T>()?;
            }
            PtrType::Raw | PtrType::FnPtr => ()
        }

        ret(())
    }
}

impl Type {
    fn check_wf<T: Target>(self) -> Result<()> {
        use Type::*;

        // Ensure that the size is valid and a multiple of the alignment.
        let size = self.size::<T>();
        ensure_wf(T::valid_size(size), "Type: size not valid")?;
        let align = self.align::<T>();
        ensure_wf(size.bytes() % align.bytes() == 0, "Type: size is not multiple of alignment")?;

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
                    ensure_wf(offset >= last_end, "Type::Tuple: overlapping fields")?;
                    last_end = offset + ty.size::<T>();
                }
                // And they must all fit into the size.
                // The size is in turn checked to be valid for `M`, and hence all offsets are valid, too.
                ensure_wf(size >= last_end, "Type::Tuple: size of fields is bigger than total size")?;
            }
            Array { elem, count } => {
                ensure_wf(count >= 0, "Type::Array: negative amount of elements")?;
                elem.check_wf::<T>()?;
            }
            Union { fields, size, chunks, align: _ } => {
                // The fields may overlap, but they must all fit the size.
                for (offset, ty) in fields {
                    ty.check_wf::<T>()?;
                    ensure_wf(
                        size >= offset + ty.size::<T>(),
                        "Type::Union: field size does not fit union",
                    )?;
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
                    ensure_wf(
                        offset >= last_end,
                        "Type::Union: chunks are not stored in ascending order",
                    )?;
                    last_end = offset + size;
                }
                // And they must all fit into the size.
                ensure_wf(size >= last_end, "Type::Union: chunks do not fit union")?;
            }
            Enum { variants, size, discriminator, discriminant_ty, .. } => {
                // All the variants need to be well-formed and be the size of the enum so
                // we don't have to handle different sizes in the memory representation.
                // Also their alignment may not be larger than the total enum alignment and
                // all the values written by the tagger must fit into the variant.
                for (discriminant, variant) in variants {
                    ensure_wf(
                        discriminant_ty.can_represent(discriminant),
                        "Type::Enum: invalid value for discriminant"
                    )?;

                    variant.ty.check_wf::<T>()?;
                    ensure_wf(
                        size == variant.ty.size::<T>(),
                        "Type::Enum: variant size is not the same as enum size"
                    )?;
                    ensure_wf(
                        variant.ty.align::<T>().bytes() <= align.bytes(),
                       "Type::Enum: invalid align requirement"
                    )?;
                    for (offset, (value_type, value)) in variant.tagger {
                        value_type.check_wf()?;
                        ensure_wf(value_type.can_represent(value), "Type::Enum: invalid tagger value")?;
                        ensure_wf(offset + value_type.size <= size, "Type::Enum tagger type size too big for enum")?;
                    }
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
    fn check_wf<T: Target>(self, size: Size, variants: Map<Int, Variant>) -> Result<()>  {
        match self {
            Discriminator::Known(discriminant) => ensure_wf(variants.get(discriminant).is_some(), "Discriminator: invalid discriminant"),
            Discriminator::Invalid => ret(()),
            Discriminator::Branch { offset, value_type, fallback, children } => {
                // Ensure that the value we branch on is stored in bounds and that all children all valid.
                value_type.check_wf()?;
                ensure_wf(offset + value_type.size <= size, "Discriminator: branch offset exceeds size")?;
                fallback.check_wf::<T>(size, variants)?;
                for (idx, ((start, end), discriminator)) in children.into_iter().enumerate() {
                    ensure_wf(value_type.can_represent(start), "Discriminator: invalid branch start bound")?;
                    // Since the end is exclusive we only need to represent the number before the end.
                    ensure_wf(value_type.can_represent(end - Int::ONE), "Discriminator: invalid branch end bound")?;
                    ensure_wf(start < end, "Discriminator: invalid bound values")?;
                    // Ensure that the ranges don't overlap.
                    ensure_wf(children.keys().enumerate().all(|(other_idx, (other_start, other_end))| 
                                other_end <= start || other_start >= end || idx == other_idx), "Discriminator: branch ranges overlap")?;
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
    fn check_wf<T: Target>(self, ty: Type, prog: Program) -> Result<()> {
        // For now, we only support integer and boolean literals and pointers.
        // TODO: add more.
        match (self, ty) {
            (Constant::Int(i), Type::Int(int_type)) => {
                ensure_wf(int_type.can_represent(i), "Constant::Int: invalid int value")?;
            }
            (Constant::Bool(_), Type::Bool) => (),
            (Constant::GlobalPointer(relocation), Type::Ptr(_)) => {
                relocation.check_wf(prog.globals)?;
            }
            (Constant::FnPointer(fn_name), Type::Ptr(_)) => {
                ensure_wf(prog.functions.contains_key(fn_name), "Constant::FnPointer: invalid function name")?;
            }
            (Constant::PointerWithoutProvenance(addr), Type::Ptr(_)) => {
                ensure_wf(
                    addr.in_bounds(Signedness::Unsigned, T::PTR_SIZE),
                    "Constant::PointerWithoutProvenance: pointer out-of-bounds"
                )?;
            }
            _ => throw_ill_formed!("Constant: value does not match type"),
        }

        ret(())
    }
}

impl ValueExpr {
    #[allow(unused_braces)]
    fn check_wf<T: Target>(self, locals: Map<LocalName, Type>, prog: Program) -> Result<Type> {
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
                        ensure_wf(exprs.len() == fields.len(), "ValueExpr::Tuple: invalid number of tuple fields")?;
                        for (e, (_offset, ty)) in exprs.zip(fields) {
                            let checked = e.check_wf::<T>(locals, prog)?;
                            ensure_wf(checked == ty, "ValueExpr::Tuple: invalid tuple field type")?;
                        }
                    },
                    Type::Array { elem, count } => {
                        ensure_wf(exprs.len() == count, "ValueExpr::Tuple: invalid number of array elements")?;
                        for e in exprs {
                            let checked = e.check_wf::<T>(locals, prog)?;
                            ensure_wf(checked == elem, "ValueExpr::Tuple: invalid array element type")?;
                        }
                    },
                    _ => throw_ill_formed!("ValueExpr::Tuple: expression does not match type"),
                }

                t
            }
            Union { field, expr, union_ty } => {
                union_ty.check_wf::<T>()?;

                let Type::Union { fields, .. } = union_ty else {
                    throw_ill_formed!("ValueExpr::Union: invalid type")
                };

                ensure_wf(field < fields.len(), "ValueExpr::Union: invalid field length")?;
                let (_offset, ty) = fields[field];

                let checked = expr.check_wf::<T>(locals, prog)?;
                ensure_wf(checked == ty, "ValueExpr::Union: invalid field type")?;

                union_ty
            }
            Variant { discriminant, data, enum_ty } => {
                let Type::Enum { variants, .. } = enum_ty else { 
                    throw_ill_formed!("ValueExpr::Variant: invalid type")
                };
                enum_ty.check_wf::<T>()?;
                let Some(variant) = variants.get(discriminant) else {
                    throw_ill_formed!("ValueExpr::Variant: invalid discriminant");
                };

                let checked = data.check_wf::<T>(locals, prog)?;
                ensure_wf(checked == variant.ty, "ValueExpr::Variant: invalid type")?;
                enum_ty
            }
            GetDiscriminant { place } => {
                let Type::Enum { discriminant_ty, .. } = place.check_wf::<T>(locals, prog)? else {
                    throw_ill_formed!("ValueExpr::GetDiscriminant: invalid type");
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
                            throw_ill_formed!("UnOp::Int: invalid operand");
                        };
                        Type::Int(int_ty)
                    }
                    Cast(cast_op) => {
                        use lang::CastOp::*;
                        match cast_op {
                            IntToInt(int_ty) => {
                                ensure_wf(matches!(operand, Type::Int(_)), "Cast::IntToInt: invalid operand")?;
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
                    Int(int_op) => {
                        let Type::Int(left) = left else {
                            throw_ill_formed!("BinOp::Int: invalid left type");
                        };
                        let Type::Int(right) = right else {
                            throw_ill_formed!("BinOp::Int: invalid right type");
                        };
                        use IntBinOp::*;
                        // Shift operators allow unequal left and right type
                        if !matches!(int_op, Shl | Shr | ShlUnchecked | ShrUnchecked) {
                            ensure_wf(left == right, "BinOp:Int: right and left type are not equal")?;
                        }
                        Type::Int(left)
                    }
                    IntWithOverflow(_int_op) => {
                        let Type::Int(int_ty) = left else {
                            throw_ill_formed!("BinOp::IntWithOverflow: invalid left type");
                        };
                        ensure_wf(right == Type::Int(int_ty), "BinOp::IntWithOverflow: invalid right type")?;
                        int_ty.with_overflow::<T>()
                    }
                    Rel(rel_op) => {
                        ensure_wf(matches!(left, Type::Int(_) | Type::Bool | Type::Ptr(_)), "BinOp::Rel: invalid left type")?;
                        ensure_wf(right == left, "BinOp::Rel: invalid right type")?;
                        match rel_op {
                            RelOp::Cmp => Type::Int(IntType::I8),
                            _ => Type::Bool,
                        }
                    }
                    PtrOffset { inbounds: _ } => {
                        ensure_wf(matches!(left, Type::Ptr(_)), "BinOp::PtrOffset: invalid left type")?;
                        ensure_wf(matches!(right, Type::Int(_)), "BinOp::PtrOffset: invalid right type")?;
                        left
                    }
                    PtrOffsetFrom { inbounds: _ } => {
                        ensure_wf(matches!(left, Type::Ptr(_)), "BinOp::PtrOffsetFrom: invalid left type")?;
                        ensure_wf(matches!(right, Type::Ptr(_)), "BinOp::PtrOffsetFrom: invalid right type")?;
                        let isize_int = IntType { signed: Signed, size: T::PTR_SIZE };
                        Type::Int(isize_int)
                    }
                }
            }
        })
    }
}

impl PlaceExpr {
    fn check_wf<T: Target>(self, locals: Map<LocalName, Type>, prog: Program) -> Result<Type> {
        use PlaceExpr::*;
        ret(match self {
            Local(name) => {
                match locals.get(name) {
                    None => throw_ill_formed!("PlaceExpr::Local: unknown local name"),
                    Some(local) => local,
                }
            },
            Deref { operand, ty } => {
                let op_ty = operand.check_wf::<T>(locals, prog)?;
                ensure_wf(matches!(op_ty, Type::Ptr(_)), "PlaceExpr::Deref: invalid type")?;
                // No check of how the alignment changes here -- that is purely a runtime constraint.
                ty
            }
            Field { root, field } => {
                let root = root.check_wf::<T>(locals, prog)?;
                let (_offset, field_ty) = match root {
                    Type::Tuple { fields, .. } | Type::Union { fields, .. } => {
                        match fields.get(field) {
                            None => throw_ill_formed!("PlaceExpr::Field: invalid field"),
                            Some(field) => field,
                        }
                    }
                    _ => throw_ill_formed!("PlaceExpr::Field: expression does not match type"),
                };
                field_ty
            }
            Index { root, index } => {
                let root = root.check_wf::<T>(locals, prog)?;
                let index = index.check_wf::<T>(locals, prog)?;
                ensure_wf(matches!(index, Type::Int(_)), "PlaceExpr::Index: invalid index type")?;
                match root {
                    Type::Array { elem, .. } => elem,
                    _ => throw_ill_formed!("PlaceExpr::Index: expression does not match Array type"),
                }
            }
            Downcast { root, discriminant } => {
                let root = root.check_wf::<T>(locals, prog)?;
                match root {
                    // A valid downcast points to an existing variant.
                    Type::Enum { variants, .. } => {
                        let Some(variant) = variants.get(discriminant) else {
                            throw_ill_formed!("PlaceExpr::Downcast: invalid discriminant");
                        };
                        variant.ty
                    }
                    _ => throw_ill_formed!("PlaceExpr::Downcast: invalid root type"),
                }
            }
        })
    }
}

impl ArgumentExpr {
    fn check_wf<T: Target>(self, locals: Map<LocalName, Type>, prog: Program) -> Result<Type> {
        ret(match self {
            ArgumentExpr::ByValue(value) => value.check_wf::<T>(locals, prog)?,
            ArgumentExpr::InPlace(place) => place.check_wf::<T>(locals, prog)?
        })
    }
}
```

## Well-formed functions and programs

```rust
impl Statement {
    fn check_wf<T: Target>(
        self,
        func: Function,
        prog: Program,
    ) -> Result<()> {
        use Statement::*;
        match self {
            Assign { destination, source } => {
                let left = destination.check_wf::<T>(func.locals, prog)?;
                let right = source.check_wf::<T>(func.locals, prog)?;
                ensure_wf(left == right, "Statement::Assign: destination and source type differ")?;
            }
            SetDiscriminant { destination, value } => {
                let Type::Enum { variants, .. } = destination.check_wf::<T>(func.locals, prog)? else {
                    throw_ill_formed!("Statement::SetDiscriminant: invalid type");
                };
                // We don't ensure that we can actually represent the discriminant.
                // The well-formedness checks for the type just ensure that every discriminant
                // reached by the discriminator is valid, however there we don't require that every
                // variant is represented. Setting such an unrepresented discriminant would probably
                // result in an invalid value as either the discriminator returns
                // `Discriminator::Invalid` or another variant.
                // This is fine as SetDiscriminant does not guarantee that the enum is a valid value.
                if variants.get(value) == None {
                    throw_ill_formed!("Statement::SetDiscriminant: invalid discriminant write")
                }
            }
            Validate { place, fn_entry: _ } => {
                place.check_wf::<T>(func.locals, prog)?;
            }
            Deinit { place } => {
                place.check_wf::<T>(func.locals, prog)?;
            }
            StorageLive(local) => {
                ensure_wf(func.locals.contains_key(local), "Statement::StorageLive: invalid local variable")?;
            }
            StorageDead(local) => {
                ensure_wf(func.locals.contains_key(local), "Statement::StorageDead: invalid local variable")?;
                if local == func.ret || func.args.any(|arg_name| local == arg_name) {
                    throw_ill_formed!("Statement::StorageDead: trying to mark argument or return local as dead");
                }
            }
        }

        ret(())
    }
}

/// Predicate to indicate if integer bin-op can be used for atomic fetch operations.
/// Needed for atomic fetch operations.
/// 
/// We limit the binops that are allowed to be atomic based on current LLVM and Rust API exposures.
fn is_atomic_binop(op: IntBinOp) -> bool {
    use IntBinOp as B;
    match op {
        B::Add | B::Sub => true,
        _ => false
    }
}

impl Terminator {
    fn check_wf<T: Target>(
        self,
        func: Function,
        prog: Program,
    ) -> Result<()> {
        use Terminator::*;
        match self {
            Goto(block_name) => {
                ensure_wf(func.blocks.contains_key(block_name), "Terminator::Goto: next block does not exist")?;
            }
            Switch { value, cases, fallback } => {
                let ty = value.check_wf::<T>(func.locals, prog)?;
                let Type::Int(switch_ty) = ty else {
                    // We only switch on integers.
                    // This is in contrast to Rust MIR where switch can work on `char`s and booleans as well.
                    // However since those are trivial casts we chose to only accept integers.
                    throw_ill_formed!("Terminator::Switch: switch is not Int")
                };

                // Ensure the switch cases are all valid.
                for (case, block) in cases.iter() {
                    ensure_wf(switch_ty.can_represent(case), "Terminator::Switch: value does not fit in switch type")?;
                    ensure_wf(func.blocks.contains_key(block), "Terminator::Switch: next block does not exist")?;
                }

                // we can also reach the fallback block.
                ensure_wf(func.blocks.contains_key(fallback), "Terminator::Switch: fallback block does not exist")?;
            }
            Unreachable => {}
            Intrinsic { intrinsic, arguments, ret, next_block } => {
                // Return and argument expressions must all typecheck with some type.
                ret.check_wf::<T>(func.locals, prog)?;
                for arg in arguments {
                    arg.check_wf::<T>(func.locals, prog)?;
                }

                // Currently only AtomicFetchAndOp has special well-formedness requirements.
                match intrinsic {
                    IntrinsicOp::AtomicFetchAndOp(op) => {
                        if !is_atomic_binop(op) {
                            throw_ill_formed!("IntrinsicOp::AtomicFetchAndOp: non atomic op");
                        }
                    }
                    _ => {}
                }

                if let Some(next_block) = next_block {
                    ensure_wf(func.blocks.contains_key(next_block), "Terminator::Call: next block does not exist")?;
                }
            }
            Call { callee, calling_convention: _, arguments, ret, next_block } => {
                let ty = callee.check_wf::<T>(func.locals, prog)?;
                ensure_wf(matches!(ty, Type::Ptr(PtrType::FnPtr)), "Terminator::Call: invalid type")?;

                // Return and argument expressions must all typecheck with some type.
                ret.check_wf::<T>(func.locals, prog)?;
                for arg in arguments {
                    arg.check_wf::<T>(func.locals, prog)?;
                }

                if let Some(next_block) = next_block {
                    ensure_wf(func.blocks.contains_key(next_block), "Terminator::Call: next block does not exist")?;
                }
            }
            Return => {}
        }

        ret(())
    }
}

impl Function {
    fn check_wf<T: Target>(self, prog: Program) -> Result<()> {
        // Ensure all locals have a valid type.
        for ty in self.locals.values() {
            ty.check_wf::<T>()?;
        }

        // Compute initially live locals: arguments and return values.
        // They must all exist and be distinct.
        let mut start_live: Set<LocalName> = Set::new();
        for arg in self.args {
            ensure_wf(self.locals.contains_key(arg), "Function: argument local does not exist")?;
            if start_live.try_insert(arg).is_err() {
                throw_ill_formed!("Function: two arguments refer to the same local");
            };
        }
        ensure_wf(self.locals.contains_key(self.ret), "Function: return local does not exist")?;
        if start_live.try_insert(self.ret).is_err() {
            throw_ill_formed!("Function: return local is also used for an argument");
        };

        // Check all basic blocks.
        for block in self.blocks.values() {
            for statement in block.statements {
                statement.check_wf::<T>(self, prog)?;
            }
            block.terminator.check_wf::<T>(self, prog)?;
        }

        ret(())
    }
}

impl Relocation {
    // Checks whether the relocation is within bounds.
    fn check_wf(self, globals: Map<GlobalName, Global>) -> Result<()> {
        // The global we are pointing to needs to exist.
        let Some(global) = globals.get(self.name) else {
            throw_ill_formed!("Relocation: invalid global name");
        };
        let size = Size::from_bytes(global.bytes.len()).unwrap();

        // And the offset needs to be in-bounds of its size.
        ensure_wf(self.offset <= size, "Relocation: offset out-of-bounds")?;

        ret(())
    }
}

impl Program {
    fn check_wf<T: Target>(self) -> Result<()> {
        // Check all the functions.
        for function in self.functions.values() {
            function.check_wf::<T>(self)?;
        }

        // Ensure the start function exists, has the right ABI, takes no arguments, and returns a 1-ZST.
        let Some(start) = self.functions.get(self.start) else {
            throw_ill_formed!("Program: start function does not exist");
        };
        ensure_wf(start.calling_convention == CallingConvention::C, "Program: start function has invalid calling convention")?;
        let ret_size = start.locals[start.ret].size::<T>();
        let ret_align = start.locals[start.ret].align::<T>();
        ensure_wf(
            ret_size == Size::ZERO && ret_align == Align::ONE,
            "Program: start function return local has invalid layout"
        )?;
        ensure_wf(start.args.is_empty(), "Program: start function has arguments")?;

        // Check globals.
        for (_name, global) in self.globals {
            let size = Size::from_bytes(global.bytes.len()).unwrap();
            for (offset, relocation) in global.relocations {
                // A relocation fills `PTR_SIZE` many bytes starting at the offset, those need to fit into the size.
                ensure_wf(offset + T::PTR_SIZE <= size, "Program: invalid global pointer value")?;

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
    fn check_wf(self, ty: Type) -> Result<()> {
        match (self, ty) {
            (Value::Int(i), Type::Int(ity)) => {
                ensure_wf(ity.can_represent(i), "Value::Int: invalid integer value")?;
            }
            (Value::Bool(_), Type::Bool) => {},
            (Value::Ptr(ptr), Type::Ptr(ptr_ty)) => {
                ensure_wf(ptr_ty.addr_valid(ptr.addr), "Value::Ptr: invalid pointer address")?;
                ensure_wf(ptr.addr.in_bounds(Unsigned, M::T::PTR_SIZE), "Value::Ptr: pointer out-of-bounds")?;
            }
            (Value::Tuple(vals), Type::Tuple { fields, .. }) => {
                ensure_wf(vals.len() == fields.len(), "Value::Tuple: invalid number of fields")?;
                for (val, (_, ty)) in vals.zip(fields) {
                    val.check_wf(ty)?;
                }
            }
            (Value::Tuple(vals), Type::Array { elem, count }) => {
                ensure_wf(vals.len() == count, "Value::Tuple: invalid number of elements")?;
                for val in vals {
                    val.check_wf(elem)?;
                }
            }
            (Value::Union(chunk_data), Type::Union { chunks, .. }) => {
                ensure_wf(chunk_data.len() == chunks.len(), "Value::Union: invalid chunk size")?;
                for (data, (_, size)) in chunk_data.zip(chunks) {
                    ensure_wf(data.len() == size.bytes(), "Value::Union: invalid chunk data")?;
                }
            }
            (Value::Variant { discriminant, data }, Type::Enum { variants, .. }) => {
                let Some(variant) = variants.get(discriminant) else {
                    throw_ill_formed!("Value::Variant: invalid discrimant type");
                };
                data.check_wf(variant.ty)?;
            }
            _ => throw_ill_formed!("Value: value does not match type")
        }

        ret(())
    }
}
```
