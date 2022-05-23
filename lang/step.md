# MiniRust Operational Semantics

This file defines the heart of MiniRust: the `step` function of the `Machine`, i.e., its operational semantics.
(To avoid having huge functions, we again use the approach of having fallible patterns in function declarations,
and having a collection of declarations with non-overlapping patterns for the same function that together cover all patterns.)

## Top-level step function

The top-level step function identifies the next terminator/statement to execute, and dispatches appropriately.
For statements it also advances the program counter.
(Terminators are themselves responsible for doing that.)

```rust
impl Machine {
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

TODO: Add unary and binary operators.

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
TODO: Actually implement the "destructive" part of this.

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

The `&` operator simply convert a place to the pointer it denotes.

```rust
impl Machine {
    fn eval_value(&mut self, Ref { target, type }: ValueExpr) -> Result<Value> {
        let p = self.eval_place(target)?;
        if !check_safe_ptr(p, type) {
            throw_ub!("creating reference to invalid (null/unaligned/uninhabited) place");
        }
        Ok(Value::Ptr(p))
    }
}
```

## Place Expressions

Place expressions evaluate to places.
For now, that is just a pointer (but this might have to change).

```rust
type Place = Pointer;
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
It also ensures that the pointer is dereferencable.

```rust
impl Machine {
    fn eval_place(&mut self, Deref(value, type): PlaceExpr) -> Result<Place> {
        let Value::Ptr(p) = self.eval_value(value)? else {
            panic!("dereferencing a non-pointer")
        };
        self.mem.dereferencable(p, type.size(), type.align())?;
        Ok(p)
    }
}
```

## Statements

Here we define how statements are evaluated.

### Assignment

Assignment evaluates its two operands, and then stores the value into the destination.
TODO: This probably needs some aliasing constraints, see [this discussion](https://github.com/rust-lang/rust/issues/68364).

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
TODO: Should we even have it, if it is equivalent?
TODO: Should this also store back the value? That would reset padding. It might also make this not equivalent to an assignment if assignment has aliasing constraints.
TODO: Should this do the job of `Retag` as well? That seems quite elegant, but might sometimes be a bit redundant.

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
        let p = self.mem.allocate(type.size(), type.align())?;
        self.cur_frame_mut().locals.try_insert(local, p).unwrap();
    }

    fn eval_statement(&mut self, StorageDead(local, type): Statement) -> Result {
        let p = self.cur_frame_mut().locals.remove(local).unwrap();
        self.mem.deallocate(p, type.size(), type.align())?;
    }
}
```

## Terminators

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

TODO: implement this

### Return

TODO: implement this
