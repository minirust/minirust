# MiniRust operators

Here we define the part of the [`step` function](step.md) that is concerned with unary and binary operators.

## Unary operators

```rust
impl Machine {
    fn eval_un_op(&mut self, operator: UnOp, operand: Value) -> Result<Value>;
}
```

### Integer operations

```rust
impl Machine {
    fn eval_un_op_int(&mut self, op: UnOpInt, operand: BigInt) -> Result<BigInt> {
        use UnOpInt::*;
        Ok(match op {
            Neg => -operand,
            Cast => operand,
        })
    }
    fn eval_un_op(&mut self, Int(op, int_type): UnOp, operand: Value) -> Result<Value> {
        let Value::Int(operand) = operand else { panic!("non-integer input to integer operation") };

        // Perform the operation.
        let result = self.eval_un_op_int(op, operand);
        // Put the result into the right range (in case of overflow).
        let result = result.modulo(int_type.signed, int_type.size);
        Ok(Value::Int(result))
    }
}
```

## Binary operators

```rust
impl Machine {
    fn eval_bin_op(&mut self, operator: BinOp, left: Value, right: Value) -> Result<Value>;
}
```

### Integer operations

```rust
impl Machine {
    fn eval_bin_op_int(&mut self, op: BinOpInt, left: BigInt, right: BigInt) -> Result<BigInt> {
        use BinOpInt::*;
        Ok(match op {
            Add => left+right,
            Sub => left-right,
        })
    }
    fn eval_bin_op(&mut self, Int(op, int_type): BinOp, left: Value, right: Value) -> Result<Value> {
        let Value::Int(left) = left else { panic!("non-integer input to integer operation") };
        let Value::Int(right) = right else { panic!("non-integer input to integer operation") };

        // Perform the operation.
        let result = self.eval_bin_op_int(op, left, right);
        // Put the result into the right range (in case of overflow).
        let result = result.modulo(int_type.signed, int_type.size);
        Ok(Value::Int(result))
    }
}
```

### Pointer arithmetic

```rust
impl Machine {
    /// Perform a wrapping offset on the given pointer. (Can never fail.)
    fn ptr_offset_wrapping(&self, ptr: Pointer, offset: BigInt) -> Pointer {
        let offset = offset.modulo(Signed, PTR_SIZE);
        let addr = ptr.addr + offset;
        let addr = addr.modulo(Unsigned, PTR_SIZE);
        Pointer { addr, ..ptr }
    }

    /// Perform in-bounds arithmetic on the given pointer. This must not wrap,
    /// and the offset must stay in bounds of a single allocation.
    fn ptr_offset_inbounds(&self, ptr: Pointer, offset: BigInt) -> Result<Pointer> {
        if !offset.in_bounds(Signed, PTR_SIZE) {
            throw_ub!("inbounds offset does not fit into `isize`):
        }
        let addr = ptr.addr + offset;
        if !addr.in_bounds(Unsigned, PTR_SIZE) {
            throw_ub!("overflowing inbounds pointer arithmetic");
        }
        let new_ptr = Pointer { addr, ..ptr };
        // We check that the range between the two pointers is dereferenceable.
        // For this, we figure out which pointer is the smaller one.
        let min_ptr = if ptr.addr <= new_ptr.addr {
            ptr
        } else {
            new_ptr
        };
        // `offset.abs()` will fit into a `Size` since we did the overflow check above.
        // FIXME: actually, it could be isize::MIN and then everything breaks? Is that
        // a valid offset?
        self.mem.dereferenceable(min_ptr, Size::from_bytes(offset.abs()).unwrap(), Align::ONE)?;
        // If this check passed, we are good.
        Ok(new_ptr)
    }

    fn eval_bin_op(&mut self, PtrOffset { inbounds }: BinOp, left: Value, right: Value) -> Result<Value> {
        let Value::Ptr(left) = left else { panic!("non-pointer left input to pointer addition") };
        let Value::Int(right) = right else { panic!("non-integer right input to pointer addition") };

        let result = if inbounds {
            self.ptr_offset_inbounds(left, right)?
        } else {
            self.ptr_offset_wrapping(left, right)
        };
        Ok(Value::Ptr(result))
    }
}
```
