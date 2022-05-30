# MiniRust Operational Semantics

This file defines the heart of MiniRust: the `step` function of the `Machine`, i.e., its operational semantics.
(To avoid having huge functions, we again use the approach of having fallible patterns in function declarations,
and having a collection of declarations with non-overlapping patterns for the same function that together cover all patterns.)

One design decision I made here is that `eval_value` and `eval_place` just return a `Value`/`Place`, but not its type.
This could be done either way, and has consequences for where in the syntax we need type annotations.
I am not sure which is better.
Miri always keeps the type with the value, so I wanted to experiment with the alternative approach and see how it goes.
The MiniRust approach has the advantage of better separating "static" from "dynamic" information.
However, I think it needs slightly more careful type-checker as part of well-formedness checking.
On the other hand, the Miri approach makes for easier to state invariants during execution.

## Top-level step function

The top-level step function identifies the next terminator/statement to execute, and dispatches appropriately.
For statements it also advances the program counter.
(Terminators are themselves responsible for doing that.)

```rust
impl Machine {
    /// To run a MiniRust program, call this in a loop until it throws an `Err` (UB or termination).
    fn step(&mut self) -> Result {
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
impl Machine {
    fn eval_value(&mut self, val: ValueExpr) -> Result<Value>;
}
```

### Constants

Constants are trivial, as one would hope.

```rust
impl Machine {
    fn eval_value(&mut self, Constant { value }: ValueExpr) -> Result<Value> {
        value
    }
}
```

### Load from memory

This loads a value from a place (often called "place-to-value coercion").

- TODO: Actually implement the "destructive" part of this.
  Also see [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/188).

```rust
impl Machine {
    fn eval_value(&mut self, Load { destructive, source, type }: ValueExpr) -> Result<Value> {
        let p = self.eval_place(source)?;
        let val = self.mem.typed_load(p, type)?;
        Ok(val)
    }
}
```

### Creating a reference/pointer

The `&` operator simply converts a place to the pointer it denotes.

```rust
impl Machine {
    fn eval_value(&mut self, Ref { target, type }: ValueExpr) -> Result<Value> {
        let p = self.eval_place(target)?;
        // We need a check here, to ensure that encoding this value at the given type is valid.
        // (For example, if the type is a reference, and this is a packed struct, it might be insufficiently aligned.)
        if !check_safe_ptr(p, type) {
            throw_ub!("creating reference to invalid (null/unaligned/uninhabited) place");
        }
        Ok(Value::Ptr(p))
    }
}
```

### Unary and binary operators

The functions `eval_un_op` and `eval_bin_op` are defined in [a separate file](operator.md).

```rust
impl Machine {
    fn eval_value(&mut self, UnOp { operator, operand }: ValueExpr) -> Result<Value> {
        let operand = self.eval_value(operand)?;
        self.eval_un_op(operator, operand)?
    }
    fn eval_value(&mut self, BinOp { operator, left, right }: ValueExpr) -> Result<Value> {
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
type Place = Pointer;

impl Machine {
    fn eval_place(&mut self, place: PlaceExpr) -> Result<Place>;
}
```

### Locals

The place for a local is directly given by the stack frame.

```rust
impl Machine {
    fn eval_place(&mut self, Local(name): PlaceExpr) -> Result<Place> {
        // This implicitly asserts that the local is live!
        Ok(self.cur_frame().locals[name])
    }
}
```

### Dereferencing a pointer

The `*` operator turns a value of pointer type into a place.
It also ensures that the pointer is dereferenceable.

- TODO: Should we truly ensure that `eval_place` always creates a dereferenceable pointer?
  Also see [this discussion](https://github.com/rust-lang/unsafe-code-guidelines/issues/319).

```rust
impl Machine {
    fn eval_place(&mut self, Deref(value, layout): PlaceExpr) -> Result<Place> {
        let Value::Ptr(p) = self.eval_value(value)? else {
            panic!("dereferencing a non-pointer")
        };
        // In case this is a raw pointer, make sure we know the place we create is dereferenceable.
        self.mem.dereferenceable(p, layout.size, layout.align)?;
        Ok(p)
    }
}
```

### Field projection

```rust
impl Machine {
    fn eval_place(&mut self, Field { root, type, field }: PlaceExpr) -> Result<Place> {
        let root = self.eval_place(root)?;
        let offset = match type {
            Tuple { fields, .. } => fields[field].0,
            Union { fields, .. } => fields[field].0,
            _ => panic!("field projection on non-projectable type"),
        };
        assert!(offset < type.size());
        // Note that the "inbounds" test here can never fail, since we ensure that
        // all places are dereferenceable. That's why we can `unwrap()`.
        Ok(self.ptr_offset_inbounds(root, offset.bytes()).unwrap())
    }
}
```

## Statements

Here we define how statements are evaluated.

```rust
impl Machine {
    fn eval_statement(&mut self, statement: Statement);
}
```

### Assignment

Assignment evaluates its two operands, and then stores the value into the destination.

- TODO: This probably needs some aliasing constraints, see [this discussion](https://github.com/rust-lang/rust/issues/68364).
- TODO: This does left-to-right evaluation. Surface Rust uses right-to-left, so we match MIR here, not Rust.
  Is that a good idea? Can we make the order not matter for UB-free executions by adding more aliasing UB?

```rust
impl Machine {
    fn eval_statement(&mut self, Assign { destination, type, source }: Statement) -> Result {
        let place = self.eval_place(destination)?;
        let val = self.eval_value(source)?;
        self.mem.typed_store(place, val, type)?;
    }
}
```

### Finalizing a value

This statement asserts that a value satisfies its validity invariant.
This is equivalent to the assignment `_ = place`.

- TODO: Should we even have it, if it is equivalent?
- TODO: Should this also store back the value? That would reset padding.
  It might also make this not equivalent to an assignment if assignment has aliasing constraints.
- TODO: Should this do the job of `Retag` as well? That seems quite elegant, but might sometimes be a bit redundant.

```rust
impl Machine {
    fn eval_statement(&mut self, Finalize { place, type }: Statement) -> Result {
        let p = self.eval_place(place)?;
        let _val = self.mem.typed_load(p, type)?;
    }
}
```

### StorageDead and StorageLive

These operations (de)allocate the memory backing a local.

```rust
impl Machine {
    fn eval_statement(&mut self, StorageLive(local, type): Statement) -> Result {
        // Here we make it a spec bug to ever mark an already live local as live.
        let p = self.mem.allocate(type.size(), type.align())?;
        self.cur_frame_mut().locals.try_insert(local, p).unwrap();
    }

    fn eval_statement(&mut self, StorageDead(local, type): Statement) -> Result {
        // Here we make it a spec bug to ever mark an already dead local as dead.
        let p = self.cur_frame_mut().locals.remove(local).unwrap();
        self.mem.deallocate(p, type.size(), type.align())?;
    }
}
```

## Terminators

```rust
impl Machine {
    fn eval_terminator(&mut self, terminator: Terminator);
}
```

### Goto

The simplest terminator: jump to the (beginning of the) given block.

```rust
impl Machine {
    fn eval_terminator(&mut self, Goto(block_name): Terminator) -> Result {
        self.cur_frame_mut().next = (block_name, 0);
    }
}
```

### If

```rust
impl Machine {
    fn eval_terminator(&mut self, If { condition, then_block, else_block }: Terminator) -> Result {
        let Value::Bool(b) = self.eval_value(condition)? else {
            panic!("if on a non-boolean")
        };
        let next = if b { then_block } else { else_block };
        self.cur_frame_mut().next = (next, 0);
    }
}
```

### Call

A lot of things happen when a function is being called!
In particular, we have to initialize the new stack frame.

- TODO: This probably needs some aliasing constraints, see [this discussion](https://github.com/rust-lang/rust/issues/71117).
- TODO: Right now, the *caller* allocates the return place. That makes `Return` very elegant, but is it truly what we want?
  In particular this means the callee cannot even tread this allocation as entirely private.
  (Aliasing will get us *some* exclusivity but not all of it.)
- TODO: This should do some kind of ABI compatibility check. Not all types with the same layout are okay to be type-punned across a call.

```rust
impl Machine {
    fn eval_terminator(&mut self, Call { callee, arguments, return_place, next_block }: Terminator) -> Result {
        let func = self.prog.functions[callee];
        // Evaluate all arguments and put them into fresh places,
        // to initialize the local variable assignment.
        if func.args.len() != arguments.len() {
            throw_ub!("call ABI violation: number of arguments does not agree");
        }
        let mut arguments: Map<LocalName, Place> =
            func.args.iter().zip(arguments.iter()).map(|(local, local_layout), (arg_val, arg_ty)| {
                let val = self.eval_value(val)?;
                // Ensure argument and local layout match.
                if local_layout != arg_ty.layout() {
                    throw_ub!("call ABI violation: argument layout does not agree");
                }
                // Allocate place and store argument value (a lot like `StorageLive`).
                let p = self.mem.allocate(local_layout.size, local_layout.align)?;
                self.mem.typed_store(p, val, ty)?;
                Ok((local, p))
            }
            .collect()?;
        // Add the return place.
        let ret_place = self.eval_place(return_place)?;
        locals.try_insert(func.ret, ret_place).unwrap();
        // Advance the PC for this stack frame.
        self.cur_frame_mut().next = (next_block, 0);
        // Push new stack frame, so it is executed next.
        self.stack.push(StackFrame {
            func,
            locals,
            next: (func.start, 0),
        });
    }
}
```

Note that the arguments and return place are entirely controlled by the caller.
The callee should probably start with a bunch of `Finalize` statements to ensure that all these arguments match the type the callee thinks they should have, and the return place is big enough.
(The latter can be done by `Finalize` with a type like `MaybeUninit<T>`.)

### Return

```rust
impl Machine {
    fn eval_terminator(&mut self, Return: Terminator) -> Result {
        let frame = self.stack.pop().unwrap();
        // Deallocate the arguments (that were allocated during `Call`).
        for (local, layout) in frame.func.args {
            // A lot like `StorageDead`.
            let p = frame.locals.remove(local).unwrap();
            self.mem.deallocate(p, layout.size, layout.align)?;
        }
        // There should be only the return local left.
        let _ret = frame.locals.remove(frame.func.ret).unwrap();
        assert!(frame.locals.is_empty());
        // The callee has already written the return place to where the caller needs it, so we are done.
    }
}
```

Note that the caller has no guarantee at all about the value that it finds in its return place.
It should probably do a `Finalize` as the next step to encode that it would be UB for the callee to return an invalid value.
