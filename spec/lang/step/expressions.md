# Expressions

This defines the evaluation of place and value expressions.

One design decision I made here is that `eval_value` and `eval_place` return both a `Value`/`Place` and its type.
Separately, [well-formedness](well-formed.md) defines `check_wf` functions that return a `Type`.
This adds some redundancy (we basically have two definitions of what the type of an expression is).
The separate `check_wf` enforces structurally that the type information is determined entirely statically.
The type propagated during evaluation means we only do a single recursive traversal, and we avoid losing track of which type a given value has (which would be a problem since a value without a type is fairly meaningless).


## Value Expressions

This section defines the following function:

```rust
impl<M: Memory> Machine<M> {
    /// Evaluate a value expression to a value. The result value will always be well-formed for the given type.
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
            Constant::PointerWithoutProvenance(addr) => {
                Value::Ptr(Pointer {
                    addr,
                    provenance: None,
                })
            }
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

### Enums

```rust
impl<M: Memory> Machine<M> {
    fn eval_value(&mut self, ValueExpr::Variant { enum_ty, discriminant, data } : ValueExpr) -> NdResult<(Value<M>, Type)> {
        ret((Value::Variant { discriminant, data: self.eval_value(data)?.0 }, enum_ty))
    }
}
```

Read the discriminant of an Enum.
The well-formedness checks already ensured that the type is an enum.

```rust
impl<M: Memory> Machine<M> {
    fn eval_value(&mut self, ValueExpr::GetDiscriminant { place } : ValueExpr) -> NdResult<(Value<M>, Type)> {
        // Get the place of the enum and its information.
        let (place, ty) = self.eval_place(place)?;
        let Type::Enum { discriminator, discriminant_ty, .. } = ty else {
            panic!("ValueExpr::GetDiscriminant requires enum type");
        };
        if !place.aligned {
            throw_ub!("Getting the discriminant of a place based on a misaligned pointer.");
        }

        // We don't require the variant to be valid,
        // we are only interested in the bytes that the discriminator actually touches.
        let accessor = |idx: Offset, size: Size| {
            let ptr = self.ptr_offset_inbounds(place.ptr, idx.bytes())?;
            // We have ensured that the place is aligned, so no alignment requirement here.
            self.mem.load(ptr, size, Align::ONE, Atomicity::None)
        };
        let Some(discriminant) = decode_discriminant::<M>(accessor, discriminator)? else {
            throw_ub!("ValueExpr::GetDiscriminant encountered invalid discriminant.");
        };

        ret((Value::Int(discriminant), Type::Int(discriminant_ty)))
    }
}
```

### Load from memory

This loads a value from a place (often called "place-to-value coercion").

```rust
impl<M: Memory> AtomicMemory<M> {
    fn place_load(&mut self, place: Place<M>, ty: Type) -> Result<Value<M>> {
        if !place.aligned {
            throw_ub!("loading from a place based on a misaligned pointer");
        }
        // Alignment was already checked.
        ret(self.typed_load(place.ptr, ty, Align::ONE, Atomicity::None)?)
    }
}

impl<M: Memory> Machine<M> {
    fn eval_value(&mut self, ValueExpr::Load { source }: ValueExpr) -> NdResult<(Value<M>, Type)> {
        let (place, ty) = self.eval_place(source)?;
        let v = self.mem.place_load(place, ty)?;

        ret((v, ty))
    }
}
```

### Creating a reference/pointer

The `&` operators simply converts a place to the pointer it denotes.

```rust
impl<M: Memory> Machine<M> {
    fn eval_value(&mut self, ValueExpr::AddrOf { target, ptr_ty }: ValueExpr) -> NdResult<(Value<M>, Type)> {
        let (place, _ty) = self.eval_place(target)?;
        // Make sure the new pointer has a valid address.
        // Remember that places are basically raw pointers so this is not guaranteed!
        if !ptr_ty.addr_valid(place.ptr.addr) {
            throw_ub!("taking the address of an invalid (null, misaligned, or uninhabited) place");
        }
        // Let the aliasing model know. (Will also check dereferenceability if appropriate.)
        let ptr = self.mem.retag_ptr(place.ptr, ptr_ty, /* fn_entry */ false)?;

        ret((Value::Ptr(ptr), Type::Ptr(ptr_ty)))
    }
}
```

### Unary and binary operators

The functions `eval_un_op` and `eval_bin_op` are defined in [a separate file](operators.md).

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
impl<M: Memory> Machine<M> {
    /// Evaluate a place expression to a place.
    ///
    /// Like a raw pointer, the result can be misaligned or null!
    #[specr::argmatch(place)]
    fn eval_place(&mut self, place: PlaceExpr) -> NdResult<(Place<M>, Type)> { .. }
}
```

One key property of place (and value) expression evaluation is that it is reorderable and removable.

### Locals

The place for a local is directly given by the stack frame.

```rust
impl<M: Memory> Machine<M> {
    fn eval_place(&mut self, PlaceExpr::Local(name): PlaceExpr) -> NdResult<(Place<M>, Type)> {
        // This implicitly asserts that the local is live!
        let ptr = self.cur_frame().locals[name];
        let ty = self.cur_frame().func.locals[name];

        ret((Place { ptr, aligned: true }, ty))
    }
}
```

### Dereferencing a pointer

The `*` operator turns a value of pointer type into a place.
It also ensures that the pointer is dereferenceable.

```rust
impl<M: Memory> Machine<M> {
    fn eval_place(&mut self, PlaceExpr::Deref { operand, ty }: PlaceExpr) -> NdResult<(Place<M>, Type)> {
        let (Value::Ptr(ptr), Type::Ptr(ptr_type)) = self.eval_value(operand)? else {
            panic!("dereferencing a non-pointer")
        };
        // We know the pointer is valid for its type, but make sure safe pointers are also dereferenceable.
        // (We don't do a full retag here, this is not considered creating a new pointer.)
        if let Some(layout) = ptr_type.safe_pointee() {
            assert!(layout.align.is_aligned(ptr.addr)); // this was already checked when the value got created
            self.mem.dereferenceable(ptr, layout.size)?;
        }
        // Check whether this pointer is sufficiently aligned.
        // Don't error immediately though! Unaligned places can still be turned into raw pointers.
        // However, they cannot be loaded from.
        let aligned = ty.align::<M::T>().is_aligned(ptr.addr);

        ret((Place { ptr, aligned }, ty))
    }
}
```

### Place projections

```rust
impl<M: Memory> Machine<M> {
    fn eval_place(&mut self, PlaceExpr::Field { root, field }: PlaceExpr) -> NdResult<(Place<M>, Type)> {
        let (root, ty) = self.eval_place(root)?;
        let (offset, field_ty) = match ty {
            Type::Tuple { fields, .. } => fields[field],
            Type::Union { fields, .. } => fields[field],
            _ => panic!("field projection on non-projectable type"),
        };
        assert!(offset <= ty.size::<M::T>());

        let ptr = self.ptr_offset_inbounds(root.ptr, offset.bytes())?;
        ret((Place { ptr, ..root }, field_ty))
    }

    fn eval_place(&mut self, PlaceExpr::Index { root, index }: PlaceExpr) -> NdResult<(Place<M>, Type)> {
        let (root, ty) = self.eval_place(root)?;
        let (Value::Int(index), _) = self.eval_value(index)? else {
            panic!("non-integer operand for array index")
        };
        let (offset, field_ty) = match ty {
            Type::Array { elem, count } => {
                if index >= 0 && index < count {
                    (index * elem.size::<M::T>(), elem)
                } else {
                    throw_ub!("out-of-bounds array access");
                }
            }
            _ => panic!("index projection on non-indexable type"),
        };
        assert!(offset <= ty.size::<M::T>());

        let ptr = self.ptr_offset_inbounds(root.ptr, offset.bytes())?;
        ret((Place { ptr, ..root }, field_ty))
    }

    fn eval_place(&mut self, PlaceExpr::Downcast { root, discriminant }: PlaceExpr) -> NdResult<(Place<M>, Type)> {
        let (root, ty) = self.eval_place(root)?;
        // We only need to downcast the enum type into the variant data type
        // since all the enum data must have the same size with offset 0 (invariant).
        let var_ty = match ty {
            Type::Enum { variants, .. } => variants[discriminant].ty,
            _ => panic!("enum downcast on non-enum"),
        };
        ret((root, var_ty))
    }
}
```
