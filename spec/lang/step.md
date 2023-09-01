# MiniRust Operational Semantics

This file defines the heart of MiniRust: the `step` function of the `Machine`, i.e., its operational semantics.
(To avoid having huge functions, we again use the approach of having fallible patterns in function declarations,
and having a collection of declarations with non-overlapping patterns for the same function that together cover all patterns.)

One design decision I made here is that `eval_value` and `eval_place` return both a `Value`/`Place` and its type.
Separately, [well-formedness](well-formed.md) defines `check_wf` functions that return a `Type`/`PlaceType`.
This adds some redundancy (we basically have two definitions of what the type of an expression is).
The separate `check_wf` enforces structurally that the type information is determined entirely statically.
The type propagated during evaluation means we only do a single recursive traversal, and we avoid losing track of which type a given value has (which would be a problem since a value without a type is fairly meaningless).

## Top-level step function

The top-level step function identifies the next terminator/statement to execute, and dispatches appropriately.
For statements it also advances the program counter.
(Terminators are themselves responsible for doing that.)

```rust
impl<M: Memory> Machine<M> {
    /// To run a MiniRust program, call this in a loop until it throws an `Err` (UB or termination).
    pub fn step(&mut self) -> NdResult {
        if !self.threads.any( |thread| thread.state == ThreadState::Enabled ) {
            throw_deadlock!();
        }

        // Reset the data race tracking *before* we change `active_thread`.
        let prev_step_information = self.reset_data_race_tracking();

        // Update current thread.
        let distr = libspecr::IntDistribution {
            start: Int::ZERO,
            end: Int::from(self.threads.len()),
            divisor: Int::ONE,
        };
        self.active_thread = pick(distr, |id: ThreadId| {
            let Some(thread) = self.threads.get(id) else {
                return false;
            };

            thread.state == ThreadState::Enabled
        })?;

        // Execute this step.
        let frame = self.cur_frame();
        let block = &frame.func.blocks[frame.next_block];
        if frame.next_stmt == block.statements.len() {
            // It is the terminator. Evaluating it will update `frame.next_block` and `frame.next_stmt`.
            self.eval_terminator(block.terminator)?;
        } else {
            // Bump up PC, evaluate this statement.
            let stmt = block.statements[frame.next_stmt];
            self.eval_statement(stmt)?;
            self.mutate_cur_frame(|frame, _mem| {
                frame.next_stmt += 1;
                ret(())
            })?;
        }

        // Check for data races with the previous step.
        self.mem.check_data_races(self.active_thread, prev_step_information)?;

        ret(())
    }

    /// Reset the data race tracking for the next step, and return the information from the previous step.
    ///
    /// The first component of the return value is the set of threads that were synchronized by the previous step,
    /// the second is the list of accesses in the previous step.
    fn reset_data_race_tracking(&mut self) -> (Set<ThreadId>, List<Access>) {
        // Remember threads synchronized by the previous step for data race detection
        // after this step.
        let mut prev_sync = self.synchronized_threads;
        // Every thread is always synchronized with itself.
        prev_sync.insert(self.active_thread);

        // Reset access tracking list.
        let prev_accesses = self.mem.reset_accesses();

        (prev_sync, prev_accesses)
    }
}
```

## Value Expressions

This section defines the following function:

```rust
impl<M: Memory> Machine<M> {
    #[specr::argmatch(val)]
    fn eval_value(&mut self, val: ValueExpr) -> NdResult<(Value<M>, Type)> { .. }
}
```

One key property of value (and place) expression evaluation is that it is reorderable and removable.
However, they are *not* deterministic due to int-to-pointer casts.

### Constants

```rust
impl<M: Memory> Machine<M> {
    /// converts `Constant` to their `Value` counterpart.
    fn eval_constant(&mut self, constant: Constant) -> Result<Value<M>> {
        ret(match constant {
            Constant::Int(i) => Value::Int(i),
            Constant::Bool(b) => Value::Bool(b),
            Constant::GlobalPointer(relocation) => {
                let ptr = self.global_ptrs[relocation.name].wrapping_offset::<M>(relocation.offset.bytes());
                Value::Ptr(ptr)
            },
            Constant::FnPointer(fn_name) => {
                Value::Ptr(Pointer {
                    addr: self.fn_addrs[fn_name],
                    provenance: None,
                })
            },
            Constant::InvalidPointer(addr) => {
                Value::Ptr(Pointer {
                    addr,
                    provenance: None,
                })
            }
            Constant::Variant { idx, data } => {
                let data = self.eval_constant(data)?;
                Value::Variant { idx, data }
            },
        })
    }

    fn eval_value(&mut self, ValueExpr::Constant(constant, ty): ValueExpr) -> NdResult<(Value<M>, Type)> {
        ret((self.eval_constant(constant)?, ty))
    }
}
```

### Tuples

```rust
impl<M: Memory> Machine<M> {
    fn eval_value(&mut self, ValueExpr::Tuple(exprs, ty): ValueExpr) -> NdResult<(Value<M>, Type)> {
        let vals = exprs.try_map(|e| self.eval_value(e))?.map(|e| e.0);
        ret((Value::Tuple(vals), ty))
    }
}
```

### Unions

```rust
impl<M: Memory> Machine<M> {
    fn eval_value(&mut self, ValueExpr::Union { field, expr, union_ty } : ValueExpr) -> NdResult<(Value<M>, Type)> {
        let Type::Union { fields, size, .. } = union_ty else { panic!("ValueExpr::Union requires union type") };
        let (offset, expr_ty) = fields[field];
        let mut data = list![AbstractByte::Uninit; size.bytes()];
        let (val, _) = self.eval_value(expr)?;
        data.write_subslice_at_index(offset.bytes(), expr_ty.encode::<M>(val));
        ret((union_ty.decode(data).unwrap(), union_ty))
    }
}
```

### Load from memory

This loads a value from a place (often called "place-to-value coercion").

```rust
impl<M: Memory> Machine<M> {
    fn eval_value(&mut self, ValueExpr::Load { source }: ValueExpr) -> NdResult<(Value<M>, Type)> {
        let (p, ptype) = self.eval_place(source)?;
        let v = self.mem.typed_load(p, ptype, Atomicity::None)?;

        ret((v, ptype.ty))
    }
}
```

### Creating a reference/pointer

The `&` operators simply converts a place to the pointer it denotes.

```rust
impl<M: Memory> Machine<M> {
    fn eval_value(&mut self, ValueExpr::AddrOf { target, ptr_ty }: ValueExpr) -> NdResult<(Value<M>, Type)> {
        let (p, _pty) = self.eval_place(target)?;
        // We generated a new pointer, let the aliasing model know.
        // FIXME: test that this is UB when the pointer requires more alignment than the place,
        // and *not* UB the other way around.
        let p = self.mem.retag_ptr(p, ptr_ty, /* fn_entry */ false)?;

        ret((Value::Ptr(p), Type::Ptr(ptr_ty)))
    }
}
```

### Unary and binary operators

The functions `eval_un_op` and `eval_bin_op` are defined in [a separate file](operator.md).

```rust
impl<M: Memory> Machine<M> {
    fn eval_value(&mut self, ValueExpr::UnOp { operator, operand }: ValueExpr) -> NdResult<(Value<M>, Type)> {
        use lang::UnOp::*;

        let operand = self.eval_value(operand)?;

        self.eval_un_op(operator, operand)
    }

    fn eval_value(&mut self, ValueExpr::BinOp { operator, left, right }: ValueExpr) -> NdResult<(Value<M>, Type)> {
        use lang::BinOp::*;

        let left = self.eval_value(left)?;
        let right = self.eval_value(right)?;


        ret(self.eval_bin_op(operator, left, right)?)
    }
}
```

## Place Expressions

Place expressions evaluate to places.
For now, that is just a pointer (but this might have to change).
Place evaluation ensures that this pointer is always dereferenceable (for the type of the place expression).

```rust
type Place<M> = Pointer<<M as Memory>::Provenance>;

impl<M: Memory> Machine<M> {
    #[specr::argmatch(place)]
    fn eval_place(&mut self, place: PlaceExpr) -> NdResult<(Place<M>, PlaceType)> { .. }
}
```

One key property of place (and value) expression evaluation is that it is reorderable and removable.

### Locals

The place for a local is directly given by the stack frame.

```rust
impl<M: Memory> Machine<M> {
    fn eval_place(&mut self, PlaceExpr::Local(name): PlaceExpr) -> NdResult<(Place<M>, PlaceType)> {
        // This implicitly asserts that the local is live!
        let place = self.cur_frame().locals[name];
        let ptype = self.cur_frame().func.locals[name];

        ret((place, ptype))
    }
}
```

### Dereferencing a pointer

The `*` operator turns a value of pointer type into a place.
It also ensures that the pointer is dereferenceable.

- TODO: Should we ensure that `eval_place` *always* creates a dereferenceable place?
  Then we could do the alignment check here, and wouldn't even have to track alignment in `PlaceType`.
  Also see [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/319).

```rust
impl<M: Memory> Machine<M> {
    fn eval_place(&mut self, PlaceExpr::Deref { operand, ptype }: PlaceExpr) -> NdResult<(Place<M>, PlaceType)> {
        let (Value::Ptr(p), Type::Ptr(ptr_type)) = self.eval_value(operand)? else {
            panic!("dereferencing a non-pointer")
        };
        // Basic check that this pointer is good for its type.
        // (We don't do a full retag here, this is not considered creating a new pointer.)
        // FIXME: test that this is UB when the pointer requires more alignment than the place,
        // and *not* UB the other way around.
        self.mem.check_pointer_dereferenceable(p, ptr_type)?;

        ret((p, ptype))
    }
}
```

### Place projections

```rust
impl<M: Memory> Machine<M> {
    fn eval_place(&mut self, PlaceExpr::Field { root, field }: PlaceExpr) -> NdResult<(Place<M>, PlaceType)> {
        let (root, ptype) = self.eval_place(root)?;
        let (offset, field_ty) = match ptype.ty {
            Type::Tuple { fields, .. } => fields[field],
            Type::Union { fields, .. } => fields[field],
            _ => panic!("field projection on non-projectable type"),
        };
        assert!(offset <= ptype.ty.size::<M::T>());

        let place = self.ptr_offset_inbounds(root, offset.bytes())?;
        let ptype = PlaceType {
            // `offset` is statically known here (it is part of the field type)
            // so we are fine using it for `ptype`.
            align: ptype.align.restrict_for_offset(offset),
            ty: field_ty,
        };

        ret((place, ptype))
    }

    fn eval_place(&mut self, PlaceExpr::Index { root, index }: PlaceExpr) -> NdResult<(Place<M>, PlaceType)> {
        let (root, ptype) = self.eval_place(root)?;
        let (Value::Int(index), _) = self.eval_value(index)? else {
            panic!("non-integer operand for array index")
        };
        let (offset, field_ty) = match ptype.ty {
            Type::Array { elem, count } => {
                if index >= 0 && index < count {
                    (index * elem.size::<M::T>(), elem)
                } else {
                    throw_ub!("out-of-bounds array access");
                }
            }
            _ => panic!("index projection on non-indexable type"),
        };
        assert!(offset <= ptype.ty.size::<M::T>());

        let place = self.ptr_offset_inbounds(root, offset.bytes())?;
        let ptype = PlaceType {
            // We do *not* use `offset` here since that is only dynamically known.
            // Instead use element size, which yields the lowest alignment.
            align: ptype.align.restrict_for_offset(field_ty.size::<M::T>()),
            ty: field_ty,
        };

        ret((place, ptype))
    }
}
```

## Statements

Here we define how statements are evaluated.

```rust
impl<M: Memory> Machine<M> {
    #[specr::argmatch(statement)]
    fn eval_statement(&mut self, statement: Statement) -> NdResult { .. }
}
```

### Assignment

Assignment evaluates its two operands, and then stores the value into the destination.

- TODO: This probably needs some aliasing constraints, see [this discussion](https://github.com/rust-lang/rust/issues/68364)
  and [this one](https://github.com/rust-lang/unsafe-code-guidelines/issues/417).
- TODO: Should this implicitly retag, to have full `Validate` semantics?

```rust
impl<M: Memory> Machine<M> {
    fn eval_statement(&mut self, Statement::Assign { destination, source }: Statement) -> NdResult {
        let (place, ptype) = self.eval_place(destination)?;
        let (val, _) = self.eval_value(source)?;
        self.mem.typed_store(place, val, ptype, Atomicity::None)?;

        ret(())
    }
}
```

### Exposing a pointer

See [this blog post](https://www.ralfj.de/blog/2022/04/11/provenance-exposed.html) for why this is needed.

```rust
impl<M: Memory> Machine<M> {
    fn eval_statement(&mut self, Statement::Expose { value }: Statement) -> NdResult {
        let (v, _type) = self.eval_value(value)?;
        let Value::Ptr(ptr) = v else { panic!("non-pointer value in `Expose`") };
        self.intptrcast.expose(ptr);

        ret(())
    }
}
```

### Validating a value

This statement asserts that a value satisfies its validity invariant, and performs retagging for the aliasing model.
(This matches the `Retag` statement in MIR. They should probaby be renamed.)

```rust
impl<M: Memory> Machine<M> {
    fn eval_statement(&mut self, Statement::Validate { place, fn_entry }: Statement) -> NdResult {
        let (p, ptype) = self.eval_place(place)?;

        let val = self.mem.typed_load(p, ptype, Atomicity::None)?;
        let val = self.mem.retag_val(val, ptype.ty, fn_entry)?;
        self.mem.typed_store(p, val, ptype, Atomicity::None)?;

        ret(())
    }
}
```

### De-initializing a place

This statement replaces the contents of a place with `Uninit`.

```rust
impl<M: Memory> Machine<M> {
    fn deinit(&mut self, place: Place<M>, pty: PlaceType) -> NdResult {
        self.mem.store(place, list![AbstractByte::Uninit; pty.ty.size::<M::T>().bytes()], pty.align, Atomicity::None)?;
        ret(())
    }

    fn eval_statement(&mut self, Statement::Deinit { place }: Statement) -> NdResult {
        let (p, ptype) = self.eval_place(place)?;
        self.deinit(p, ptype)?;

        ret(())
    }
}
```

### StorageDead and StorageLive

These operations (de)allocate the memory backing a local.

```rust
impl<M: Memory> StackFrame<M> {
    fn storage_live(&mut self, mem: &mut AtomicMemory<M>, local: LocalName) -> NdResult {
        let layout = self.func.locals[local].layout::<M::T>();
        let p = mem.allocate(AllocationKind::Stack, layout.size, layout.align)?;
        // Here we make it a spec bug to ever mark an already live local as live.
        self.locals.try_insert(local, p).unwrap();
        ret(())
    }

    fn storage_dead(&mut self, mem: &mut AtomicMemory<M>, local: LocalName) -> NdResult {
        let layout = self.func.locals[local].layout::<M::T>();
        let p = self.locals.remove(local).unwrap();
        // Here we make it a spec bug to ever mark an already dead local as dead.
        // FIXME: This does not match what rustc does: https://github.com/rust-lang/rust/issues/98896.
        mem.deallocate(p, AllocationKind::Stack, layout.size, layout.align)?;
        ret(())
    }
}

impl<M: Memory> Machine<M> {
    fn eval_statement(&mut self, Statement::StorageLive(local): Statement) -> NdResult {
        self.mutate_cur_frame(|frame, mem| {
            frame.storage_live(mem, local)
        })
    }

    fn eval_statement(&mut self, Statement::StorageDead(local): Statement) -> NdResult {
        self.mutate_cur_frame(|frame, mem| {
            frame.storage_dead(mem, local)
        })
    }
}
```

## Terminators

```rust
impl<M: Memory> Machine<M> {
    #[specr::argmatch(terminator)]
    fn eval_terminator(&mut self, terminator: Terminator) -> NdResult { .. }
}
```

### Goto

The simplest terminator: jump to the (beginning of the) given block.

```rust
impl<M: Memory> Machine<M> {
    fn jump_to_block(&mut self, block: BbName) -> NdResult {
        self.mutate_cur_frame(|frame, _mem| {
            frame.jump_to_block(block);
            ret(())
        })
    }

    fn eval_terminator(&mut self, Terminator::Goto(block_name): Terminator) -> NdResult {
        self.jump_to_block(block_name)?;
        ret(())
    }
}
```

### If

```rust
impl<M: Memory> Machine<M> {
    fn eval_terminator(&mut self, Terminator::If { condition, then_block, else_block }: Terminator) -> NdResult {
        let (Value::Bool(b), _) = self.eval_value(condition)? else {
            panic!("if on a non-boolean")
        };
        let next = if b { then_block } else { else_block };
        self.jump_to_block(next)?;

        ret(())
    }
}
```

### Unreachable

```rust
impl<M: Memory> Machine<M> {
    fn eval_terminator(&mut self, Terminator::Unreachable: Terminator) -> NdResult {
        throw_ub!("reached unreachable code");
    }
}
```

### Call

A lot of things happen when a function is being called!
In particular, we have to initialize the new stack frame.

```rust
/// Check whether the two types are compatible in function calls.
///
/// This means *at least* they have the same size and alignment (for on-stack argument passing).
/// However, when arguments get passed in registers, more details become relevant, so we require
/// almost full structural equality.
fn check_abi_compatibility(
    caller_pty: PlaceType,
    callee_pty: PlaceType,
) -> bool {
    // FIXME: we probably do not have enough details captured in `Type` to fully implement this.
    // For instance, what about SIMD vectors?
    // FIXME: we also reject too much here, e.g. we do not reflect `repr(transparent)`,
    // let alone `Option<&T>` being compatible with `*const T`.
    fn check_ty_abi_compatibility(
        caller_ty: Type,
        callee_ty: Type,
    ) -> bool {
        match (caller_ty, callee_ty) {
            (Type::Int(caller_ty), Type::Int(callee_ty)) =>
                // Signedness does not matter for ABI
                caller_ty.size == callee_ty.size,
            (Type::Bool, Type::Bool) =>
                true,
            (Type::Ptr(_), Type::Ptr(_)) =>
                // The kind of pointer and pointee details do not matter for ABI.
                true,
            (Type::Tuple { fields: caller_fields, size: caller_size },
             Type::Tuple { fields: callee_fields, size: callee_size }) =>
                caller_fields.len() == callee_fields.len() &&
                caller_fields.zip(callee_fields).all(|(caller_field, callee_field)|
                    caller_field.0 == callee_field.0 && check_ty_abi_compatibility(caller_field.1, callee_field.1)
                ) &&
                caller_size == callee_size,
            (Type::Array { elem: caller_elem, count: caller_count },
             Type::Array { elem: callee_elem, count: callee_count }) =>
                check_ty_abi_compatibility(caller_elem, callee_elem) && caller_count == callee_count,
            (Type::Union { fields: caller_fields, chunks: caller_chunks, size: caller_size },
             Type::Union { fields: callee_fields, chunks: callee_chunks, size: callee_size }) =>
                caller_fields.len() == callee_fields.len() &&
                caller_fields.zip(callee_fields).all(|(caller_field, callee_field)|
                    caller_field.0 == callee_field.0 && check_ty_abi_compatibility(caller_field.1, callee_field.1)
                ) &&
                caller_chunks == callee_chunks &&
                caller_size == callee_size,
            (Type::Enum { variants: caller_variants, tag_encoding: caller_encoding, size: caller_size },
             Type::Enum { variants: callee_variants, tag_encoding: callee_encoding, size: callee_size }) =>
                caller_variants.len() == callee_variants.len() &&
                caller_variants.zip(callee_variants).all(|(caller_field, callee_field)|
                    check_ty_abi_compatibility(caller_field, callee_field)
                ) &&
                caller_encoding == callee_encoding &&
                caller_size == callee_size,
            // Different kind of type, definitely incompatible.
            _ =>
                false
        }
    }
    caller_pty.align == callee_pty.align && check_ty_abi_compatibility(caller_pty.ty, callee_pty.ty)
}

impl<M: Memory> Machine<M> {
    /// A helper function to deal with `ArgumentExpr`.
    fn eval_argument(
        &mut self,
        val: ArgumentExpr,
    ) -> NdResult<(Value<M>, PlaceType)> {
        ret(match val {
            ArgumentExpr::ByValue(value, align) => {
                let (value, ty) = self.eval_value(value)?;
                (value, PlaceType::new(ty, align))
            }
            ArgumentExpr::InPlace(place) => {
                let (place, pty) = self.eval_place(place)?;
                let value = self.mem.typed_load(place, pty, Atomicity::None)?;
                // Make the old value unobservable because the callee might work on it in-place.
                // FIXME: This also needs aliasing model support.
                self.deinit(place, pty)?;

                (value, pty)
            }
        })
    }

    /// Creates a stack frame for the given function, initializes the arguments,
    /// and ensures that calling convention and argument/return value ABIs are all matching up.
    fn create_frame(
        &mut self,
        func: Function,
        return_action: ReturnAction<M>,
        caller_conv: CallingConvention,
        caller_ret_pty: PlaceType,
        caller_args: List<(Value<M>, PlaceType)>,
    ) -> NdResult<StackFrame<M>> {
        let mut frame = StackFrame {
            func,
            locals: Map::new(),
            return_action,
            next_block: func.start,
            next_stmt: Int::ZERO,
        };

        // Allocate all the initially live locals.
        frame.storage_live(&mut self.mem, func.ret)?;
        for arg_local in func.args {
            frame.storage_live(&mut self.mem, arg_local)?;
        }

        // Check calling convention.
        if caller_conv != func.calling_convention {
            throw_ub!("call ABI violation: calling conventions are not the same");
        }

        // Check return place compatibility.
        if !check_abi_compatibility(caller_ret_pty, func.locals[func.ret]) {
            throw_ub!("call ABI violation: return types are not compatible");
        }

        // Pass arguments and check their compatibility.
        if func.args.len() != caller_args.len() {
            throw_ub!("call ABI violation: number of arguments does not agree");
        }
        for (callee_local, (caller_val, caller_pty)) in func.args.zip(caller_args) {
            // Make sure caller and callee view of this are compatible.
            if !check_abi_compatibility(caller_pty, func.locals[callee_local]) {
                throw_ub!("call ABI violation: argument types are not compatible");
            }
            // Copy the value at caller (source) type -- that's necessary since it is the type the values come at.
            // We know the types have compatible layout so this will fit into the allocation.
            // The local is freshly allocated so there should be no reason the store can fail.
            self.mem.typed_store(frame.locals[callee_local], caller_val, caller_pty, Atomicity::None).unwrap();
        }

        ret(frame)
    }

    fn eval_terminator(
        &mut self,
        Terminator::Call { callee, arguments, ret: ret_expr, next_block }: Terminator
    ) -> NdResult {
        // First evaluate the return place and remember it for `Return`. (Left-to-right!)
        let (caller_ret_place, caller_ret_pty) = self.eval_place(ret_expr)?;
        // To allow in-place return value passing, we proactively make the old contents
        // of the return place unobservable.
        // FIXME: This also needs aliasing model support.
        self.deinit(caller_ret_place, caller_ret_pty)?;

        // Then evaluate the function that will be called.
        let (Value::Ptr(ptr), Type::Ptr(PtrType::FnPtr(caller_conv))) = self.eval_value(callee)? else {
            panic!("call on a non-pointer")
        };
        let func = self.fn_from_addr(ptr.addr)?;

        // Then evaluate the arguments.
        let arguments = arguments.try_map(|arg| self.eval_argument(arg))?;

        // Set up the stack frame.
        let return_action = ReturnAction::ReturnToCaller {
            next_block,
            ret_place: caller_ret_place,
        };
        let frame = self.create_frame(
            func,
            return_action,
            caller_conv,
            caller_ret_pty,
            arguments,
        )?;

        // Push new stack frame, so it is executed next.
        self.mutate_cur_stack(|stack| stack.push(frame));
        ret(())
    }
}
```

Note that the content of the arguments is entirely controlled by the caller.
The callee should probably start with a bunch of `Validate` statements to ensure that all these arguments match the type the callee thinks they should have.

### Return

```rust
impl<M: Memory> Machine<M> {
    fn terminate_active_thread(&mut self) -> NdResult {
        let active = self.active_thread;
        // We would habe reached UB before the main thread terminates.
        assert!(active != 0, "the main thread cannot terminate");

        self.threads.mutate_at(active, |thread| {
            assert!(thread.stack.len() == 0);
            thread.state = ThreadState::Terminated;
        });

        // All threads that waited to join this thread get synchronized by this termination
        // and enabled again.
        for i in ThreadId::ZERO..self.threads.len() {
            if self.threads[i].state == ThreadState::BlockedOnJoin(active) {
                self.synchronized_threads.insert(i);
                self.threads.mutate_at(i, |thread| thread.state = ThreadState::Enabled)
            }
        }

        ret(())
    }

    fn eval_terminator(&mut self, Terminator::Return: Terminator) -> NdResult {
        let mut frame = self.mutate_cur_stack(
            |stack| stack.pop().unwrap()
        );

        // Load the return value.
        // To match `Call`, and since the callee might have written to its return place using a totally different type,
        // we copy at the callee (source) type -- the one place where we ensure the return value matches that type.
        let ret_pty = frame.func.locals[frame.func.ret];
        let ret_val = self.mem.typed_load(frame.locals[frame.func.ret], ret_pty, Atomicity::None)?;

        // Deallocate everything.
        while let Some(local) = frame.locals.keys().next() {
            frame.storage_dead(&mut self.mem, local)?;
        }

        // Perform the return action.
        match frame.return_action {
            ReturnAction::BottomOfStack => {
                // Only the bottom frame in a stack has no caller.
                // Therefore the thread must terminate now.
                self.terminate_active_thread()?;
            }
            ReturnAction::ReturnToCaller { ret_place: caller_ret_place, next_block } => {
                // There must be a caller.
                assert!(self.active_thread().stack.len() > 0);
                // Store the return value where the caller wanted it.
                // Crucially, we are doing the store at the same type as the load above.
                self.mem.typed_store(caller_ret_place, ret_val, ret_pty, Atomicity::None)?;
                // Jump to where the caller wants us to jump.
                if let Some(next_block) = next_block {
                    self.jump_to_block(next_block)?;
                } else {
                    throw_ub!("return from a function where caller did not specify next block");
                }
            }
        }

        ret(())
    }
}
```

Note that the caller has no guarantee at all about the value that it finds in its return place.
It should probably do a `Validate` as the next step to encode that it would be UB for the callee to return an invalid value.

### Intrinsic

```rust
impl<M: Memory> Machine<M> {
    fn eval_terminator(
        &mut self,
        Terminator::CallIntrinsic { intrinsic, arguments, ret: ret_expr, next_block }: Terminator
    ) -> NdResult {
        // First evaluate return place (left-to-right evaluation).
        let (ret_place, ret_pty) = self.eval_place(ret_expr)?;

        // Evaluate all arguments.
        let arguments = arguments.try_map(|arg| self.eval_value(arg))?;

        // Run the actual intrinsic.
        let value = self.eval_intrinsic(intrinsic, arguments, ret_pty.ty)?;

        // Store return value.
        // `eval_inrinsic` above must guarantee that `value` has the right type.
        self.mem.typed_store(ret_place, value, ret_pty, Atomicity::None)?;

        // Jump to next block.
        if let Some(next_block) = next_block {
            self.jump_to_block(next_block)?;
        } else {
            throw_ub!("return from an intrinsic where caller did not specify next block");
        }

        ret(())
    }
}
```
