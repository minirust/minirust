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
