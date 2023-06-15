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
        let distr = libspecr::IntDistribution {
            start: Int::ZERO,
            end: Int::from(self.thread_manager.threads.len()),
            divisor: Int::ONE,
        };

        let thread_id: ThreadId = pick(distr, |id: ThreadId| {
            let Some(thread) = self.thread_manager.threads.get(id) else {
                return false;
            };

            thread.state == ThreadState::Enabled
        })?;

        self.thread_manager.active_thread = Some(thread_id);

        let frame = self.cur_frame();
        let block = &frame.func.blocks[frame.next_block];
        if frame.next_stmt == block.statements.len() {
            // It is the terminator. Evaluating it will update `frame.next_block` and `frame.next_stmt`.
            self.eval_terminator(block.terminator)?;
        } else {
            // Bump up PC, evaluate this statement.
            let stmt = block.statements[frame.next_stmt];
            self.mutate_cur_frame(|frame| {
                frame.next_stmt += 1;
            });
            self.eval_statement(stmt)?;
        }

        ret(())
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
    fn eval_value(&mut self, ValueExpr::Load { destructive, source }: ValueExpr) -> NdResult<(Value<M>, Type)> {
        let (p, ptype) = self.eval_place(source)?;
        let v = self.mem.typed_load(Atomicity::None, p, ptype)?;
        if destructive {
            // Overwrite the source with `Uninit`.
            self.mem.store(Atomicity::None, p, list![AbstractByte::Uninit; ptype.ty.size::<M>().bytes()], ptype.align)?;
        }

        ret((v, ptype.ty))
    }
}
```

### Creating a reference/pointer

The `&` operators simply converts a place to the pointer it denotes.

```rust
impl<M: Memory> Machine<M> {
    fn eval_value(&mut self, ValueExpr::AddrOf { target, ptr_ty }: ValueExpr) -> NdResult<(Value<M>, Type)> {
        let (p, _) = self.eval_place(target)?;
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

TODO: In almost all cases, callers also need to compute the type of this place, so maybe it should be returned from `eval_place`?
It is a bit annoying to keep in sync with `check_wf`, but for Coq it would be much better to avoid recursing over the `PlaceExpr` twice.

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

- TODO: Should we ensure that `eval_place` always creates a dereferenceable place?
  Then we could do the alignment check here, and wouldn't even have to track alignment in `PlaceType`.
  Also see [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/319).

```rust
impl<M: Memory> Machine<M> {
    fn eval_place(&mut self, PlaceExpr::Deref { operand, ptype }: PlaceExpr) -> NdResult<(Place<M>, PlaceType)> {
        let (Value::Ptr(p), _) = self.eval_value(operand)? else {
            panic!("dereferencing a non-pointer")
        };

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
        assert!(offset <= ptype.ty.size::<M>());

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
                    (index * elem.size::<M>(), elem)
                } else {
                    throw_ub!("out-of-bounds array access");
                }
            }
            _ => panic!("index projection on non-indexable type"),
        };
        assert!(offset <= ptype.ty.size::<M>());

        let place = self.ptr_offset_inbounds(root, offset.bytes())?;
        let ptype = PlaceType {
            // We do *not* use `offset` here since that is only dynamically known.
            align: ptype.align.restrict_for_offset(field_ty.size::<M>()),
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

- TODO: This probably needs some aliasing constraints, see [this discussion](https://github.com/rust-lang/rust/issues/68364).
- TODO: This does left-to-right evaluation. Surface Rust uses right-to-left, so we match MIR here, not Rust.
  Is that a good idea? Maybe we should impose some syntactic restrictions to ensure that the evaluation order does not matter, such as:
  - If there is a destructive load in either expression, then there must be no other load.
  - If there is a ptr2int cast, then there must be no int2ptr cast.

    Or maybe we should change the grammar to make these cases impossible (like, make ptr2int casts proper statements). Also we have to assume that reads in the memory model can be reordered.

```rust
impl<M: Memory> Machine<M> {
    fn eval_statement(&mut self, Statement::Assign { destination, source }: Statement) -> NdResult {
        let (place, ptype) = self.eval_place(destination)?;
        let (val, _) = self.eval_value(source)?;
        self.mem.typed_store(Atomicity::None, place, val, ptype)?;

        ret(())
    }
}
```

### Finalizing a value

This statement asserts that a value satisfies its validity invariant, and performs retagging for the aliasing model.

- TODO: Should `Retag` be a separate operation instead?

```rust
impl<M: Memory> Machine<M> {
    fn eval_statement(&mut self, Statement::Finalize { place, fn_entry }: Statement) -> NdResult {
        let (p, ptype) = self.eval_place(place)?;

        let val = self.mem.typed_load(Atomicity::None, p, ptype)?;
        let val = self.mem.retag_val(val, ptype.ty, fn_entry)?;
        self.mem.typed_store(Atomicity::None, p, val, ptype)?;

        ret(())
    }
}
```

### StorageDead and StorageLive

These operations (de)allocate the memory backing a local.

```rust
impl<M: Memory> Machine<M> {
    fn eval_statement(&mut self, Statement::StorageLive(local): Statement) -> NdResult {
        // Here we make it a spec bug to ever mark an already live local as live.
        let layout = self.cur_frame().func.locals[local].layout::<M>();
        let p = self.mem.allocate(layout.size, layout.align)?;
        self.mutate_cur_frame(|frame| {
            frame.locals.try_insert(local, p).unwrap();
        });

        ret(())
    }

    fn eval_statement(&mut self, Statement::StorageDead(local): Statement) -> NdResult {
        // Here we make it a spec bug to ever mark an already dead local as dead.
        let layout = self.cur_frame().func.locals[local].layout::<M>();
        let p = self.mutate_cur_frame(|frame| {
            frame.locals.remove(local).unwrap()
        });
        self.mem.deallocate(p, layout.size, layout.align)?;

        ret(())
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
    fn eval_terminator(&mut self, Terminator::Goto(block_name): Terminator) -> NdResult {
        self.mutate_cur_frame(|frame| {
            frame.jump_to_block(block_name);
        });

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
        self.mutate_cur_frame(|frame| {
            frame.jump_to_block(next);
        });

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

- TODO: This probably needs some aliasing constraints, see [this discussion](https://github.com/rust-lang/rust/issues/71117).

```rust
impl<M: Memory> Machine<M> {
    fn eval_terminator(
        &mut self,
        Terminator::Call { callee, arguments, ret: ret_expr, next_block }: Terminator
    ) -> NdResult {
        let mut locals: Map<LocalName, Place<M>> = Map::new();

        // First evaluate the return place and remember it for `Return`. (Left-to-right!)
        let ret_place = ret_expr.try_map(|(caller_ret_place, _abi)| {
            self.eval_place(caller_ret_place)
        })?;

        // Then evaluate the function that will be called.
        let (Value::Ptr(ptr), _) = self.eval_value(callee)? else {
            panic!("call on a non-pointer")
        };

        let func = self.fn_from_addr(ptr.addr)?;

        // Create place for return local, if needed.
        if let Some((ret_local, _abi)) = func.ret {
            let callee_ret_layout = func.locals[ret_local].layout::<M>();
            locals.insert(ret_local, self.mem.allocate(callee_ret_layout.size, callee_ret_layout.align)?);
        }

        // Check ABI compatibility.
        if let (Some((_, caller_ret_abi)), Some((_, callee_ret_abi))) = (ret_expr, func.ret) {
            if caller_ret_abi != callee_ret_abi {
                throw_ub!("call ABI violation: return ABI does not agree");
            }
        } else {
            // FIXME: Can we truly accept any caller/callee ABI if the other respective ABI is missing?
        }

        // Evaluate all arguments and put them into fresh places,
        // to initialize the local variable assignment.
        if func.args.len() != arguments.len() {
            throw_ub!("call ABI violation: number of arguments does not agree");
        }
        for ((local, callee_abi), (arg, caller_abi)) in func.args.zip(arguments) {
            let (val, caller_ty) = self.eval_value(arg)?;
            let callee_layout = func.locals[local].layout::<M>();
            if caller_abi != callee_abi {
                throw_ub!("call ABI violation: argument ABI does not agree");
            }
            // Allocate place with callee layout (a lot like `StorageLive`).
            let p = self.mem.allocate(callee_layout.size, callee_layout.align)?;
            // Store value with caller type (otherwise we could get panics).
            // The ABI above should ensure that this does not go OOB,
            // and it is a fresh pointer so there should be no other reason this can fail.
            self.mem.typed_store(Atomicity::None, p, val, PlaceType::new(caller_ty, callee_layout.align)).unwrap();
            locals.insert(local, p);
        }

        // Push new stack frame, so it is executed next.
        self.mutate_cur_stack(|stack| stack.push(StackFrame {
            func,
            locals,
            caller_return_info: Some(CallerReturnInfo {
                next_block,
                ret_place,
            }),
            next_block: func.start,
            next_stmt: Int::ZERO,
        }));

        ret(())
    }
}
```

Note that the content of the arguments is entirely controlled by the caller.
The callee should probably start with a bunch of `Finalize` statements to ensure that all these arguments match the type the callee thinks they should have.

### Return

```rust
impl<M: Memory> Machine<M> {
    fn eval_terminator(&mut self, Terminator::Return: Terminator) -> NdResult {
        let frame = self.mutate_cur_stack(
            |stack| stack.pop().unwrap()
        );
        let func = frame.func;

        let Some(caller_return_info) = frame.caller_return_info else {
            // Only the bottom frame in a stack has no caller.
            // Therefore the thread must terminate now.
            assert_eq!(Int::ZERO, self.thread_manager.cur_thread().stack.len());

            return self.thread_manager.terminate_active_thread();
        };

        let Some((ret_local, _)) = func.ret else {
            throw_ub!("return from a function that does not have a return local");
        };

        // Copy return value, if any, to where the caller wants it.
        // To make this work like an assignment in the caller, we use the caller type for this copy.
        // FIXME: Should Call/Return also do a copy at *callee* type?
        // On the other hand, the callee is done here, so why would it even still
        // care about the return value?
        if let Some((ret_place, ret_pty)) = caller_return_info.ret_place {
            let ret_val = self.mem.typed_load(Atomicity::None, frame.locals[ret_local], ret_pty)?;
            self.mem.typed_store(Atomicity::None, ret_place, ret_val, ret_pty)?;
        }

        // Deallocate everything.
        for (local, place) in frame.locals {
            // A lot like `StorageDead`.
            let layout = func.locals[local].layout::<M>();
            self.mem.deallocate(place, layout.size, layout.align)?;
        }

        if let Some(next_block) = caller_return_info.next_block {
            self.mutate_cur_frame(|frame| {
                frame.jump_to_block(next_block);
            });
        } else {
            throw_ub!("return from a function where caller did not specify next block");
        }

        ret(())
    }
}
```

Note that the caller has no guarantee at all about the value that it finds in its return place.
It should probably do a `Finalize` as the next step to encode that it would be UB for the callee to return an invalid value.

### Intrinsic

```rust
impl<M: Memory> Machine<M> {
    fn eval_terminator(
        &mut self,
        Terminator::CallIntrinsic { intrinsic, arguments, ret: ret_expr, next_block }: Terminator
    ) -> NdResult {
        // First evaluate return place (left-to-right evaluation).
        let ret_place = ret_expr.try_map(|ret_expr| self.eval_place(ret_expr))?;

        // Evaluate all arguments.
        let arguments = arguments.try_map(|arg| self.eval_value(arg))?;

        let ret_ty = ret_place.map(|(_, pty)| pty.ty).unwrap_or_else(|| unit_type());

        let value = self.eval_intrinsic(intrinsic, arguments, ret_ty)?;

        if let Some((ret_place, ret_pty)) = ret_place {
            // `eval_inrinsic` above must guarantee that `value` has the right type.
            self.mem.typed_store(Atomicity::None, ret_place, value, ret_pty)?;
        }

        if let Some(next_block) = next_block {
            self.mutate_cur_frame(|frame| {
                frame.jump_to_block(next_block);
            });
        } else {
            throw_ub!("return from an intrinsic where caller did not specify next block");
        }

        ret(())
    }
}
```
