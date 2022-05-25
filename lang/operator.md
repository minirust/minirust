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
    fn eval_un_op(&mut self, INeg(int_type): UnOp, operand: Value) -> Result<Value> {
        let Value::Int(operand) = operand else { panic!("non-integer input to integer operation") };

        let result = (-operand).modulo(int_type.signed, int_type.size);
        Ok(Value::Int(result))
    }

    fn eval_un_op(&mut self, ICast { to: int_type }: UnOp, operand: Value) -> Result<Value> {
        let Value::Int(operand) = operand else { panic!("non-integer input to integer operation") };

        let result = operand.modulo(int_type.signed, int_type.size);
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
    fn eval_bin_op(&mut self, IAdd(int_type): BinOp, left: Value, right: Value) -> Result<Value> {
        let Value::Int(left) = left else { panic!("non-integer input to integer operation") };
        let Value::Int(right) = right else { panic!("non-integer input to integer operation") };

        let result = (left+right).modulo(int_type.signed, int_type.size);
        Ok(Value::Int(result))
    }
}
```
