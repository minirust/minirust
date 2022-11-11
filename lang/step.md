# MiniRust Operational Semantics

This file defines the heart of MiniRust: the `step` function of the `Machine`, i.e., its operational semantics.
(To avoid having huge functions, we again use the approach of having fallible patterns in function declarations,
and having a collection of declarations with non-overlapping patterns for the same function that together cover all patterns.)

One design decision I made here is that `eval_value` and `eval_place` just return a `Value`/`Place`, but not its type.
Separately, [well-formedness](well-formed.md) defines `check_wf` functions that return a `Type`/`PlaceType`.
This adds some redundancy, but makes also enforces structurally that the type information is determined entirely statically.
(In the future, when we translate this to Coq, we might want to make `eval_value`/`eval_place` additionally return the type, to avoid doing two recursive traversals of the expression.)

## Top-level step function

The top-level step function identifies the next terminator/statement to execute, and dispatches appropriately.
For statements it also advances the program counter.
(Terminators are themselves responsible for doing that.)

```rust
impl<M: Memory> Machine<M> {
    /// To run a MiniRust program, call this in a loop until it throws an `Err` (UB or termination).
    fn step(&mut self) -> NdResult {
        let frame = self.cur_frame_mut();
        let (next_block, next_stmt) = &mut frame.next;
        let block = &frame.func.blocks[next_block];
        if next_stmt == block.statements.len() {
            // It is the terminator.
            self.eval_terminator(block.terminator)?;
        } else {
            // Bump up PC, evaluate this statement.
            let stmt = block.statements[next_stmt];
            next_stmt += 1;
            self.eval_statement(stmt)?;
        }
    }
}
```

## Value Expressions

This section defines the following function:

```rust
impl<M: Memory> Machine<M> {
    #[specr::argmatch(val)]
    fn eval_value(&mut self, val: ValueExpr) -> NdResult<Value<M>>;
}
```

### Constants

```rust
impl<M: Memory> Machine<M> {
    /// converts `Constant` to their `Value` counterpart.
    fn eval_constant(&mut self, constant: Constant) -> NdResult<Value<M>> {
        match constant {
            Constant::Int(i) => Value::Int(i),
            Constant::Bool(b) => Value::Bool(b),
            Constant::Tuple(args) => {
                let vals = args.into_iter()
                    .map(|c| self.eval_constant(c))
                    .try_collect()?;
                Value::Tuple(vals)
            },
            Constant::Variant { idx, data } => {
                let data = self.eval_constant(data)?;
                Value::Variant { idx, data }
            },
        }
    }

    fn eval_value(&mut self, ValueExpr::Constant(constant, _ty): ValueExpr) -> NdResult<Value<M>> {
        self.eval_constant(constant)?
    }
}
```

### Load from memory

This loads a value from a place (often called "place-to-value coercion").

```rust
impl<M: Memory> Machine<M> {
    fn eval_value(&mut self, ValueExpr::Load { destructive, source }: ValueExpr) -> NdResult<Value<M>> {
        let p = self.eval_place(source)?;
        let ptype = source.check_wf::<M>(self.cur_frame().func.locals).unwrap(); // FIXME avoid a second traversal of `source`
        let v = self.mem.typed_load(p, ptype)?;
        if destructive {
            // Overwrite the source with `Uninit`.
            self.mem.store(p, list![AbstractByte::Uninit; ptype.ty.size::<M>()], ptype.align)?;
        }
        v
    }
}
```

### Creating a reference/pointer

The `&` operators simply converts a place to the pointer it denotes.

```rust
impl<M: Memory> Machine<M> {
    fn eval_value(&mut self, ValueExpr::AddrOf { target, .. }: ValueExpr) -> NdResult<Value<M>> {
        let p = self.eval_place(target)?;
        Value::Ptr(p)
    }
}
```

### Unary and binary operators

The functions `eval_un_op` and `eval_bin_op` are defined in [a separate file](operator.md).

```rust
impl<M: Memory> Machine<M> {
    fn eval_value(&mut self, ValueExpr::UnOp { operator, operand }: ValueExpr) -> NdResult<Value<M>> {
        let operand = self.eval_value(operand)?;
        self.eval_un_op(operator, operand)?
    }
    fn eval_value(&mut self, ValueExpr::BinOp { operator, left, right }: ValueExpr) -> NdResult<Value<M>> {
        let left = self.eval_value(left)?;
        let right = self.eval_value(right)?;
        self.eval_bin_op(operator, left, right)?
    }
}
```

## Place Expressions

Place expressions evaluate to places.
For now, that is just a pointer (but this might have to change).
Place evaluation ensures that this pointer is always dereferenceable (for the type of the place expression).

```rust
type Place<M: Memory> = Pointer<M::Provenance>;

impl<M: Memory> Machine<M> {
    #[specr::argmatch(place)]
    fn eval_place(&mut self, place: PlaceExpr) -> NdResult<Place<M>>;
}
```

TODO: In almost all cases, callers also need to compute the type of this place, so maybe it should be returned from `eval_place`?
It is a bit annoying to keep in sync with `check_wf`, but for Coq it would be much better to avoid recursing over the `PlaceExpr` twice.

### Locals

The place for a local is directly given by the stack frame.

```rust
impl<M: Memory> Machine<M> {
    fn eval_place(&mut self, PlaceExpr::Local(name): PlaceExpr) -> NdResult<Place<M>> {
        // This implicitly asserts that the local is live!
        self.cur_frame().locals[name]
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
    fn eval_place(&mut self, PlaceExpr::Deref { operand, .. }: PlaceExpr) -> NdResult<Place<M>> {
        let Value::Ptr(p) = self.eval_value(operand)? else {
            panic!("dereferencing a non-pointer")
        };
        p
    }
}
```

### Place projections

```rust
impl<M: Memory> Machine<M> {
    fn eval_place(&mut self, PlaceExpr::Field { root, field }: PlaceExpr) -> NdResult<Place<M>> {
        let ty = root.check_wf::<M>(self.cur_frame().func.locals).unwrap().ty; // FIXME avoid a second traversal of `root`
        let root = self.eval_place(root)?;
        let offset = match ty {
            Type::Tuple { fields, .. } => fields[field].0,
            Type::Union { fields, .. } => fields[field].0,
            _ => panic!("field projection on non-projectable type"),
        };
        assert!(offset < ty.size::<M>());
        self.ptr_offset_inbounds(root, offset.bytes())?
    }

    fn eval_place(&mut self, PlaceExpr::Index { root, index }: PlaceExpr) -> NdResult<Place<M>> {
        let ty = root.check_wf::<M>(self.cur_frame().func.locals).unwrap().ty; // FIXME avoid a second traversal of `root`
        let root = self.eval_place(root)?;
        let Value::Int(index) = self.eval_value(index)? else {
            panic!("non-integer operand for array index")
        };
        let offset = match ty {
            Type::Array { elem, count } => {
                if index < count {
                    index * elem.size::<M>()
                } else {
                    throw_ub!("out-of-bounds array access");
                }
            }
            _ => panic!("index projection on non-indexable type"),
        };
        assert!(offset < ty.size::<M>());
        self.ptr_offset_inbounds(root, offset.bytes())?
    }
}
```

## Statements

Here we define how statements are evaluated.

```rust
impl<M: Memory> Machine<M> {
    #[specr::argmatch(statement)]
    fn eval_statement(&mut self, statement: Statement) -> NdResult;
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
        let place = self.eval_place(destination)?;
        let val = self.eval_value(source)?;
        let ptype = destination.check_wf::<M>(self.cur_frame().func.locals).unwrap(); // FIXME avoid a second traversal of `destination`
        self.mem.typed_store(place, val, ptype)?;
    }
}
```

### Finalizing a value

This statement asserts that a value satisfies its validity invariant, and performs retagging for the aliasing model.

- TODO: Should `Retag` be a separate operation instead?

```rust
impl<M: Memory> Machine<M> {
    fn eval_statement(&mut self, Statement::Finalize { place, fn_entry }: Statement) -> NdResult {
        let p = self.eval_place(place)?;
        let ptype = place.check_wf::<M>(self.cur_frame().func.locals).unwrap(); // FIXME avoid a second traversal of `place`
        let val = self.mem.typed_load(p, ptype)?;
        let val = self.mem.retag_val(val, ptype.ty, fn_entry)?;
        self.mem.typed_store(p, val, ptype)?;
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
        self.cur_frame_mut().locals.try_insert(local, p).unwrap();
    }

    fn eval_statement(&mut self, Statement::StorageDead(local): Statement) -> NdResult {
        // Here we make it a spec bug to ever mark an already dead local as dead.
        let layout = self.cur_frame().func.locals[local].layout::<M>();
        let p = self.cur_frame_mut().locals.remove(local).unwrap();
        self.mem.deallocate(p, layout.size, layout.align)?;
    }
}
```

## Terminators

```rust
impl<M: Memory> Machine<M> {
    #[specr::argmatch(terminator)]
    fn eval_terminator(&mut self, terminator: Terminator) -> NdResult;
}
```

### Goto

The simplest terminator: jump to the (beginning of the) given block.

```rust
impl<M: Memory> Machine<M> {
    fn eval_terminator(&mut self, Terminator::Goto(block_name): Terminator) -> NdResult {
        self.cur_frame_mut().next = (block_name, 0);
    }
}
```

### If

```rust
impl<M: Memory> Machine<M> {
    fn eval_terminator(&mut self, Terminator::If { condition, then_block, else_block }: Terminator) -> NdResult {
        let Value::Bool(b) = self.eval_value(condition)? else {
            panic!("if on a non-boolean")
        };
        let next = if b { then_block } else { else_block };
        self.cur_frame_mut().next = (next, 0);
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
        Terminator::Call { callee, arguments, ret, next_block }: Terminator
    ) -> NdResult {
        let Some(func) = self.prog.functions.get(callee) else {
            throw_ub!("calling non-existing function");
        };
        let mut locals: Map<LocalName, Place<M>> = default();

        // First evaluate the return place. (Left-to-right!)
        // Create place for return local.
        let (ret_local, callee_ret_abi) = func.ret;
        let callee_ret_layout = func.locals[ret_local].layout::<M>();
        locals.insert(ret_local, self.mem.allocate(callee_ret_layout.size, callee_ret_layout.align)?);
        // Remember the return place (will be relevant during `Return`).
        let (caller_ret_place, caller_ret_abi) = ret;
        let caller_ret_place = self.eval_place(caller_ret_place)?;
        if caller_ret_abi != callee_ret_abi {
            throw_ub!("call ABI violation: return ABI does not agree");
        }

        // Evaluate all arguments and put them into fresh places,
        // to initialize the local variable assignment.
        if func.args.len() != arguments.len() {
            throw_ub!("call ABI violation: number of arguments does not agree");
        }
        for ((local, callee_abi), (arg, caller_abi)) in func.args.iter().zip(arguments.iter()) {
            let val = self.eval_value(arg)?;
            let caller_ty = arg.check_wf::<M>(func.locals).unwrap(); // FIXME avoid a second traversal of `arg`
            let callee_layout = func.locals[local].layout::<M>();
            if caller_abi != callee_abi {
                throw_ub!("call ABI violation: argument ABI does not agree");
            }
            // Allocate place with callee layout (a lot like `StorageLive`).
            let p = self.mem.allocate(callee_layout.size, callee_layout.align)?;
            // Store value with caller type (otherwise we could get panics).
            // The size check above should ensure that this does not go OOB,
            // and it is a fresh pointer so there should be no other reason this can fail.
            self.mem.typed_store(p, val, PlaceType::new(caller_ty, callee_layout.align)).unwrap();
            locals.insert(local, p);
        }

        // Advance the PC for this stack frame.
        self.cur_frame_mut().next = (next_block, 0);
        // Push new stack frame, so it is executed next.
        self.stack.push(StackFrame {
            func,
            locals,
            caller_ret_place,
            next: (func.start, 0),
        });
    }
}
```

Note that the content of the arguments is entirely controlled by the caller.
The callee should probably start with a bunch of `Finalize` statements to ensure that all these arguments match the type the callee thinks they should have.

### Return

```rust
impl<M: Memory> Machine<M> {
    fn eval_terminator(&mut self, Terminator::Return: Terminator) -> NdResult {
        let frame = self.stack.pop().unwrap();
        let func = frame.func;
        // Copy return value to where the caller wants it.
        // We use the type as given by `func` here (callee type) as otherwise we
        // would never ensure that the value is valid at that type.
        let ret_pty = func.locals[func.ret.0];
        let ret_val = self.mem.typed_load(frame.locals[func.ret.0], ret_pty)?;
        self.mem.typed_store(frame.caller_ret_place, ret_val, ret_pty)?;
        // Deallocate everything.
        for (local, place) in frame.locals {
            // A lot like `StorageDead`.
            let layout = func.locals[local].layout::<M>();
            self.mem.deallocate(place, layout.size, layout.align)?;
        }
    }
}
```

Note that the caller has no guarantee at all about the value that it finds in its return place.
It should probably do a `Finalize` as the next step to encode that it would be UB for the callee to return an invalid value.
