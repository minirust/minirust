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
    fn eval_int_un_op(op: IntUnOp, operand: Int) -> Result<Int> {
        use IntUnOp::*;
        ret(match op {
            Neg => -operand,
            BitNot => !operand,
        })
    }
    fn eval_un_op(&self, UnOp::Int(op): UnOp, (operand, op_ty): (Value<M>, Type)) -> Result<(Value<M>, Type)> {
        let Type::Int(int_ty) = op_ty else { panic!("non-integer input to integer operation") };
        let Value::Int(operand) = operand else { panic!("non-integer input to integer operation") };

        // Perform the operation.
        let result = Self::eval_int_un_op(op, operand)?;
        // Put the result into the right range (in case of overflow).
        let result = int_ty.bring_in_bounds(result);
        ret((Value::Int(result), Type::Int(int_ty)))
    }
}
```

### Casts

```rust
impl<M: Memory> Machine<M> {
    fn eval_cast_op(cast_op: CastOp, (operand, old_ty): (Value<M>, Type)) -> Result<(Value<M>, Type)> {
        use CastOp::*;
        match cast_op {
            IntToInt(int_ty) => {
                let Value::Int(operand) = operand else { panic!("non-integer input to int-to-int cast") };
                let result = int_ty.bring_in_bounds(operand);
                ret((Value::Int(result), Type::Int(int_ty)))
            }
            Transmute(new_ty) => {
                // Question: Why is this not a well-formedness constraint?
                // `core::intrinsics::transmute` states:
                // "Both types must have the same size. Compilation will fail if this is not guaranteed."
                if old_ty.size::<M::T>().unwrap_size() != new_ty.size::<M::T>().unwrap_size() {
                    throw_ub!("transmute between types of different size")
                }
                let Some(val) = transmute(operand, old_ty, new_ty) else {
                    throw_ub!("transmuted value is not valid at new type")
                };
                ret((val, new_ty))
            }
        }
    }
    fn eval_un_op(&self, UnOp::Cast(cast_op): UnOp, (operand, op_ty): (Value<M>, Type)) -> Result<(Value<M>, Type)> {
        ret(Self::eval_cast_op(cast_op, (operand, op_ty))?)
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
    fn eval_int_bin_op(op: IntBinOp, left: Int, right: Int, left_ty: IntType) -> Result<Int> {
        use IntBinOp::*;
        ret(match op {
            Add => left + right,
            AddUnchecked => {
                let result = left + right;
                if !left_ty.can_represent(result) {
                    throw_ub!("overflow in unchecked add");
                }
                result
            }
            Sub => left - right,
            SubUnchecked => {
                let result = left - right;
                if !left_ty.can_represent(result) {
                    throw_ub!("overflow in unchecked sub");
                }
                result
            }
            Mul => left * right,
            MulUnchecked => {
                let result = left * right;
                if !left_ty.can_represent(result) {
                    throw_ub!("overflow in unchecked mul");
                }
                result
            }
            Div => {
                if right == 0 {
                    throw_ub!("division by zero");
                }
                let result = left / right;
                if !left_ty.can_represent(result) { // `int::MIN / -1` is UB
                    throw_ub!("overflow in division");
                }
                result
            }
            Rem => {
                if right == 0 {
                    throw_ub!("modulus of remainder is zero");
                }
                if !left_ty.can_represent(left / right) { // `int::MIN % -1` is UB
                    throw_ub!("overflow in remainder");
                }
                left % right
            }
            Shl | Shr => {
                let bits = left_ty.size.bits();
                let offset = right.rem_euclid(bits);

                match op {
                    Shl => left << offset,
                    Shr => left >> offset,
                    _ => panic!(),
                }
            }
            ShlUnchecked | ShrUnchecked => {
                let bits = left_ty.size.bits();
                if right < 0 || right >= bits {
                    throw_ub!("overflow in unchecked shift");
                }

                match op {
                    ShlUnchecked => left << right,
                    ShrUnchecked => left >> right,
                    _ => panic!(),
                }
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
        let result = Self::eval_int_bin_op(op, left, right, int_ty)?;
        // Put the result into the right range (in case of overflow).
        let result = int_ty.bring_in_bounds(result);
        ret((Value::Int(result), Type::Int(int_ty)))
    }

    fn eval_bin_op(
        &self,
        BinOp::IntWithOverflow(op): BinOp,
        (left, l_ty): (Value<M>, Type),
        (right, _r_ty): (Value<M>, Type)
    ) -> Result<(Value<M>, Type)> {
        let Type::Int(int_ty) = l_ty else { panic!("non-integer input to integer operation") };
        let Value::Int(left) = left else { panic!("non-integer input to integer operation") };
        let Value::Int(right) = right else { panic!("non-integer input to integer operation") };

        // Perform the operation.
        let result = match op {
            IntBinOpWithOverflow::Add => left + right,
            IntBinOpWithOverflow::Sub => left - right,
            IntBinOpWithOverflow::Mul => left * right,
        };
        let overflow = !int_ty.can_represent(result);
        // Put the result into the right range (in case of overflow).
        let result = int_ty.bring_in_bounds(result);
        // Pack result and overflow bool into tuple.
        let value = Value::Tuple(list![Value::Int::<M>(result), Value::Bool::<M>(overflow)]);
        let ty = int_ty.with_overflow::<M::T>();
        ret((value, ty))
    }
}
```

### Relational operators

```rust
impl<M: Memory> Machine<M> {
    fn eval_rel_op(rel: RelOp, left: Int, right: Int) -> (Value<M>, Type) {
        use RelOp::*;
        let b = match rel {
            Lt => left < right,
            Gt => left > right,
            Le => left <= right,
            Ge => left >= right,
            Eq => left == right,
            Ne => left != right,
            Cmp => {
                // 3-way comparison.
                let result = if left < right {
                    Int::from(-1)
                } else if left == right {
                    Int::from(0)
                } else {
                    Int::from(1)
                };

                // FIXME avoid using early return
                return (Value::Int(result), Type::Int(IntType::I8));
            }
        };
        (Value::Bool(b), Type::Bool)
    }
    /// Prepare a value for being compared with a `RelOp`. This involves turning it into an `Int`.
    fn prepare_for_rel_op(val: Value<M>) -> Int {
        match val {
            Value::Int(i) => i,
            Value::Bool(b) => if b { Int::from(1) } else { Int::from(0) },
            Value::Ptr(p) => p.thin_pointer.addr,
            _ => panic!("invalid value for relational operator"),
        }
    }
    fn eval_bin_op(
        &self,
        BinOp::Rel(rel_op): BinOp,
        (left, l_ty): (Value<M>, Type),
        (right, _r_ty): (Value<M>, Type)
    ) -> Result<(Value<M>, Type)> {
        let left = Self::prepare_for_rel_op(left);
        let right = Self::prepare_for_rel_op(right);

        ret(Self::eval_rel_op(rel_op, left, right))
    }
}
```

### Pointer arithmetic

```rust
impl<M: Memory> Machine<M> {
    /// Perform a wrapping offset on the given pointer. (Can never fail.)
    fn ptr_offset_wrapping(&self, ptr: ThinPointer<M::Provenance>, offset: Int) -> ThinPointer<M::Provenance> {
        ptr.wrapping_offset::<M::T>(offset)
    }

    /// Perform in-bounds arithmetic on the given pointer. This must not wrap,
    /// and the offset must stay in bounds of a single allocation.
    fn ptr_offset_inbounds(&self, ptr: ThinPointer<M::Provenance>, offset: Int) -> Result<ThinPointer<M::Provenance>> {
        // Ensure dereferenceability. This also ensures that `offset` fits in an `isize`, since no allocation
        // can be bigger than `isize`, and it ensures that the arithmetic does not overflow, since no
        // allocation wraps around the edge of the address space.
        self.mem.signed_dereferenceable(ptr, offset)?;
        // All checked!
        ret(ThinPointer { addr: ptr.addr + offset, ..ptr })
    }

    fn eval_bin_op(
        &self,
        BinOp::PtrOffset { inbounds }: BinOp,
        (left, l_ty): (Value<M>, Type),
        (right, _r_ty): (Value<M>, Type)
    ) -> Result<(Value<M>, Type)> {
        let Value::Ptr(Pointer { thin_pointer: left, metadata: left_meta }) = left else {
            panic!("non-pointer left input to `PtrOffsetFrom`")
        };
        let Value::Int(right) = right else { panic!("non-integer right input to `PtrOffset`") };

        let offset_ptr = if inbounds {
            self.ptr_offset_inbounds(left, right)?
        } else {
            self.ptr_offset_wrapping(left, right)
        };
        ret((Value::Ptr(offset_ptr.widen(left_meta)), l_ty))
    }

    fn eval_bin_op(
        &self,
        BinOp::PtrOffsetFrom { inbounds }: BinOp,
        (left, l_ty): (Value<M>, Type),
        (right, _r_ty): (Value<M>, Type)
    ) -> Result<(Value<M>, Type)> {
        let Value::Ptr(Pointer { thin_pointer: left, .. }) = left else {
            panic!("non-pointer left input to `PtrOffsetFrom`")
        };
        let Value::Ptr(Pointer { thin_pointer: right, .. }) = right else {
            panic!("non-pointer right input to `PtrOffsetFrom`")
        };

        let distance = left.addr - right.addr;
        let distance = if inbounds {
            // The "gap" between the two pointers must be dereferenceable from both of them.
            // This check also ensures that the distance is inbounds of `isize`.
            self.mem.signed_dereferenceable(left, -distance)?;
            self.mem.signed_dereferenceable(right, distance)?;
            // All checked!
            distance
        } else {
            distance.bring_in_bounds(Signed, M::T::PTR_SIZE)
        };

        let isize_int = IntType { signed: Signed, size: M::T::PTR_SIZE };
        ret((Value::Int(distance), Type::Int(isize_int)))
    }
}
```
