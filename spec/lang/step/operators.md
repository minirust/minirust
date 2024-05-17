# Operators

Here we define the evaluation of unary and binary operators.

## Unary operators

```rust
impl<M: Memory> Machine<M> {
    #[specr::argmatch(operator)]
    fn eval_un_op(&self, operator: UnOp, (operand, op_ty): (Value<M>, Type)) -> Result<(Value<M>, Type)> { .. }
}
```

### Integer operations

```rust
impl<M: Memory> Machine<M> {
    fn eval_un_op_int(&self, op: UnOpInt, operand: Int) -> Result<Int> {
        use UnOpInt::*;
        ret(match op {
            Neg => -operand,
            Not => !operand,
        })
    }
    fn eval_un_op(&self, UnOp::Int(op): UnOp, (operand, op_ty): (Value<M>, Type)) -> Result<(Value<M>, Type)> {
        let Type::Int(int_ty) = op_ty else { panic!("non-integer input to integer operation") };
        let Value::Int(operand) = operand else { panic!("non-integer input to integer operation") };

        // Perform the operation.
        let result = self.eval_un_op_int(op, operand)?;
        // Put the result into the right range (in case of overflow).
        let result = int_ty.bring_in_bounds(result);
        ret((Value::Int(result), Type::Int(int_ty)))
    }
}
```

### Boolean operations

```rust
impl<M: Memory> Machine<M> {
    fn eval_un_op_bool(&self, op: UnOpBool, operand: bool) -> bool {
        use UnOpBool::*;
        match op {
            Not => !operand,
        }
    }
    fn eval_un_op(&self, UnOp::Bool(op): UnOp, (operand, op_ty): (Value<M>, Type)) -> Result<(Value<M>, Type)> {
        let Value::Bool(value) = operand else { panic!("non-boolean input to boolean unop") };
        let result = self.eval_un_op_bool(op, value);
        ret((Value::Bool(result), Type::Bool))
    }
}
```

### Casts

```rust
impl<M: Memory> Machine<M> {
    fn eval_cast(&self, cast_op: CastOp, (operand, old_ty): (Value<M>, Type)) -> Result<(Value<M>, Type)> {
        use CastOp::*;
        match cast_op {
            IntToInt(int_ty) => {
                let Value::Int(operand) = operand else { panic!("non-integer input to int-to-int cast") };
                let result = int_ty.bring_in_bounds(operand);
                ret((Value::Int(result), Type::Int(int_ty)))
            }
            BoolToInt(int_ty) => {
                let Value::Bool(operand) = operand else { panic!("non-boolean input to bool-to-int cast") };
                // 0 and 1 fit into every `IntTy`, so we do not need to `bring_in_bounds`.
                let result = if operand { Int::ONE } else { Int::ZERO };
                ret((Value::Int(result), Type::Int(int_ty)))
            },
            Transmute(new_ty) => {
                if old_ty.size::<M::T>() != new_ty.size::<M::T>() {
                    throw_ub!("transmute between types of different size")
                };
                let Some(val) = transmute(operand, old_ty, new_ty) else {
                    throw_ub!("transmuted value is not valid at new type")
                };
                ret((val, new_ty))
            }
        }
    }
    fn eval_un_op(&self, UnOp::Cast(cast_op): UnOp, (operand, op_ty): (Value<M>, Type)) -> Result<(Value<M>, Type)> {
        ret(self.eval_cast(cast_op, (operand, op_ty))?)
    }
}
```

## Binary operators

```rust
impl<M: Memory> Machine<M> {
    #[specr::argmatch(operator)]
    fn eval_bin_op(
        &self,
        operator: BinOp,
        (left, l_ty):
        (Value<M>, Type),
        (right, _r_ty): (Value<M>, Type)
    ) -> Result<(Value<M>, Type)> { .. }
}
```

### Integer operations

```rust
impl<M: Memory> Machine<M> {
    fn eval_bin_op_int(&self, op: BinOpInt, left: Int, right: Int) -> Result<Int> {
        use BinOpInt::*;
        ret(match op {
            Add => left + right,
            Sub => left - right,
            Mul => left * right,
            Div => {
                if right == 0 {
                    throw_ub!("division by zero");
                }
                left / right
            }
            Rem => {
                if right == 0 {
                    throw_ub!("modulus of remainder is zero");
                }
                left % right
            }
            BitAnd => left & right,
            BitOr => left | right,
            BitXor => left ^ right,
        })
    }
    fn eval_bin_op(
        &self,
        BinOp::Int(op): BinOp,
        (left, l_ty): (Value<M>, Type),
        (right, _r_ty): (Value<M>, Type)
    ) -> Result<(Value<M>, Type)> {
        let Type::Int(int_ty) = l_ty else { panic!("non-integer input to integer operation") };
        let Value::Int(left) = left else { panic!("non-integer input to integer operation") };
        let Value::Int(right) = right else { panic!("non-integer input to integer operation") };

        // Perform the operation.
        let result = self.eval_bin_op_int(op, left, right)?;
        // Put the result into the right range (in case of overflow).
        let result = int_ty.bring_in_bounds(result);
        ret((Value::Int(result), Type::Int(int_ty)))
    }
}
```

### Integer relations

```rust
impl<M: Memory> Machine<M> {
    fn eval_int_rel(&self, rel: IntRel, left: Int, right: Int) -> bool {
        use IntRel::*;
        match rel {
            Lt => left < right,
            Gt => left > right,
            Le => left <= right,
            Ge => left >= right,
            Eq => left == right,
            Ne => left != right,
        }
    }
    fn eval_bin_op(
        &self,
        BinOp::IntRel(int_rel): BinOp,
        (left, l_ty): (Value<M>, Type),
        (right, _r_ty): (Value<M>, Type)
    ) -> Result<(Value<M>, Type)> {
        let Value::Int(left) = left else { panic!("non-integer input to integer relation") };
        let Value::Int(right) = right else { panic!("non-integer input to integer relation") };

        let result = self.eval_int_rel(int_rel, left, right);
        ret((Value::Bool(result), Type::Bool))
    }
}
```

### Pointer arithmetic

```rust
impl<M: Memory> Machine<M> {
    /// Perform a wrapping offset on the given pointer. (Can never fail.)
    fn ptr_offset_wrapping(&self, ptr: Pointer<M::Provenance>, offset: Int) -> Pointer<M::Provenance> {
        ptr.wrapping_offset::<M>(offset)
    }

    /// Perform in-bounds arithmetic on the given pointer. This must not wrap,
    /// and the offset must stay in bounds of a single allocation.
    fn ptr_offset_inbounds(&self, ptr: Pointer<M::Provenance>, offset: Int) -> Result<Pointer<M::Provenance>> {
        if !offset.in_bounds(Signed, M::T::PTR_SIZE) {
            throw_ub!("inbounds offset does not fit into `isize`");
        }
        let addr = ptr.addr + offset;
        if !addr.in_bounds(Unsigned, M::T::PTR_SIZE) {
            throw_ub!("overflowing inbounds pointer arithmetic");
        }
        let new_ptr = Pointer { addr, ..ptr };
        // TODO: Do we even want this 'dereferenceable' restriction?
        // See <https://github.com/rust-lang/unsafe-code-guidelines/issues/350>.
        // We check that the range between the two pointers is dereferenceable.
        // For this, we figure out which pointer is the smaller one.
        let min_ptr = if ptr.addr <= new_ptr.addr {
            ptr
        } else {
            new_ptr
        };
        // `offset.abs()` is obviously positive, hence `unwrap()`.
        self.mem.dereferenceable(min_ptr, Size::from_bytes(offset.abs()).unwrap())?;
        // If this check passed, we are good.
        ret(new_ptr)
    }

    fn eval_bin_op(
        &self,
        BinOp::PtrOffset { inbounds }: BinOp,
        (left, l_ty): (Value<M>, Type),
        (right, _r_ty): (Value<M>, Type)
    ) -> Result<(Value<M>, Type)> {
        let Value::Ptr(left) = left else { panic!("non-pointer left input to pointer addition") };
        let Value::Int(right) = right else { panic!("non-integer right input to pointer addition") };

        let result = if inbounds {
            self.ptr_offset_inbounds(left, right)?
        } else {
            self.ptr_offset_wrapping(left, right)
        };
        ret((Value::Ptr(result), l_ty))
    }
}
```

### Boolean operations

```rust
impl<M: Memory> Machine<M> {
    fn eval_bin_op_bool(&self, op: BinOpBool, left: bool, right: bool) -> bool {
        use BinOpBool::*;
        match op {
            BitAnd => left & right,
            BitOr => left | right,
            BitXor => left ^ right,
        }
    }
    fn eval_bin_op(
        &self,
        BinOp::Bool(op): BinOp,
        (left, l_ty): (Value<M>, Type),
        (right, _r_ty): (Value<M>, Type)
    ) -> Result<(Value<M>, Type)> {
        let Value::Bool(left) = left else { panic!("non-boolean input to boolean binop") };
        let Value::Bool(right) = right else { panic!("non-boolean input to boolean binop") };

        // Perform the operation.
        let result = self.eval_bin_op_bool(op, left, right);
        ret((Value::Bool(result), Type::Bool))
    }
}
```
