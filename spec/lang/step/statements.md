
# Statements

Here we define how statements are evaluated.

```rust
impl<M: Memory> Machine<M> {
    #[specr::argmatch(statement)]
    fn eval_statement(&mut self, statement: Statement) -> NdResult { .. }
}
```

## Assignment

Assignment evaluates its two operands, and then stores the value into the destination.

- TODO: This probably needs some aliasing constraints, see [this discussion](https://github.com/rust-lang/rust/issues/68364)
  and [this one](https://github.com/rust-lang/unsafe-code-guidelines/issues/417).
- TODO: Should this implicitly retag, to have full `Validate` semantics?

```rust
impl<M: Memory> AtomicMemory<M> {
    fn place_store(&mut self, place: Place<M>, val: Value<M>, ty: Type) -> Result {
        if !place.aligned {
            throw_ub!("storing to a place based on a misaligned pointer");
        }
        // Alignment was already checked.
        self.typed_store(place.ptr, val, ty, Align::ONE, Atomicity::None)?;
        ret(())
    }
}

impl<M: Memory> Machine<M> {
    fn eval_statement(&mut self, Statement::Assign { destination, source }: Statement) -> NdResult {
        let (place, ty) = self.eval_place(destination)?;
        let (val, _) = self.eval_value(source)?;
        self.mem.place_store(place, val, ty)?;

        ret(())
    }
}
```

## Exposing a pointer

See [this blog post](https://www.ralfj.de/blog/2022/04/11/provenance-exposed.html) for why this is needed.

```rust
impl<M: Memory> Machine<M> {
    fn eval_statement(&mut self, Statement::Expose { value }: Statement) -> NdResult {
        let (v, _type) = self.eval_value(value)?;
        let Value::Ptr(ptr) = v else { panic!("non-pointer value in `Expose`") };
        self.intptrcast.expose(ptr);

        ret(())
    }
}
```

## Setting a discriminant

```rust
impl<M: Memory> Machine<M> {
    fn eval_statement(&mut self, Statement::SetDiscriminant { destination, value }: Statement) -> NdResult {
        let (Place { ptr, aligned: true }, Type::Enum { variants, size, align, .. }) = self.eval_place(destination)? else {
            panic!("Setting the discriminant type of a non-enum contradicts well-formedness.");
        };
        let (Value::Int(idx), _int_ty) = self.eval_value(value)? else {
            panic!("Setting the discriminant to a non-int type contradicts well-formedness.");
        };
        let tagger = match variants.get(idx) {
            Some(Variant { tagger, .. }) => tagger,
            None => throw_ub!("Setting an invalid discriminant ({idx})"),
        };

        // Load the bytes from memory, store tag into the bytes and write the bytes back to memory.
        // This should be fine as we don't allow encoded data and the tag to overlap for valid enum variants.
        let mut bytes = self.mem.load(ptr, size, align, Atomicity::None)?;
        for (offset, value) in tagger.iter() {
            bytes.set(offset.bytes(), AbstractByte::Init(value, None));
        }
        self.mem.store(ptr, bytes, align, Atomicity::None)?;
        ret(())
    }
}
```

## Validating a value

This statement asserts that a value satisfies its validity invariant, and performs retagging for the aliasing model.
(This matches the `Retag` statement in MIR. They should probaby be renamed.)

```rust
impl<M: Memory> Machine<M> {
    fn eval_statement(&mut self, Statement::Validate { place, fn_entry }: Statement) -> NdResult {
        let (place, ty) = self.eval_place(place)?;

        let val = self.mem.place_load(place, ty)?;
        let val = self.mem.retag_val(val, ty, fn_entry)?;
        self.mem.place_store(place, val, ty)?;

        ret(())
    }
}
```

## De-initializing a place

This statement replaces the contents of a place with `Uninit`.

```rust
impl<M: Memory> AtomicMemory<M> {
    fn deinit(&mut self, ptr: Pointer<M::Provenance>, len: Size, align: Align) -> Result {
        self.store(ptr, list![AbstractByte::Uninit; len.bytes()], align, Atomicity::None)?;
        ret(())
    }
}

impl<M: Memory> Machine<M> {
    fn eval_statement(&mut self, Statement::Deinit { place }: Statement) -> NdResult {
        let (p, ty) = self.eval_place(place)?;
        if !p.aligned {
            throw_ub!("de-initializing a place based on a misaligned pointer");
        }
        // Alignment was already checked.
        self.mem.deinit(p.ptr, ty.size::<M::T>(), Align::ONE)?;

        ret(())
    }
}
```

## StorageDead and StorageLive

These operations (de)allocate the memory backing a local.

```rust
impl<M: Memory> StackFrame<M> {
    fn storage_live(&mut self, mem: &mut AtomicMemory<M>, local: LocalName) -> NdResult {
        let layout = self.func.locals[local].layout::<M::T>();
        let ptr = mem.allocate(AllocationKind::Stack, layout.size, layout.align)?;
        // Here we make it a spec bug to ever mark an already live local as live.
        self.locals.try_insert(local, ptr).unwrap();
        ret(())
    }

    fn storage_dead(&mut self, mem: &mut AtomicMemory<M>, local: LocalName) -> NdResult {
        let layout = self.func.locals[local].layout::<M::T>();
        let ptr = self.locals.remove(local).unwrap();
        // Here we make it a spec bug to ever mark an already dead local as dead.
        // FIXME: This does not match what rustc does: https://github.com/rust-lang/rust/issues/98896.
        mem.deallocate(ptr, AllocationKind::Stack, layout.size, layout.align)?;
        ret(())
    }
}

impl<M: Memory> Machine<M> {
    fn eval_statement(&mut self, Statement::StorageLive(local): Statement) -> NdResult {
        self.mutate_cur_frame(|frame, mem| {
            frame.storage_live(mem, local)
        })
    }

    fn eval_statement(&mut self, Statement::StorageDead(local): Statement) -> NdResult {
        self.mutate_cur_frame(|frame, mem| {
            frame.storage_dead(mem, local)
        })
    }
}
```
