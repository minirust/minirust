
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
impl<M: Memory> ConcurrentMemory<M> {
    fn place_store(&mut self, place: Place<M>, val: Value<M>, ty: Type) -> Result {
        if !place.aligned {
            throw_ub!("storing to a place based on a misaligned pointer");
        }
        // Alignment was already checked.
        self.typed_store(place.ptr.thin_pointer, val, ty, Align::ONE, Atomicity::None)?;
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

## Setting a discriminant

```rust
impl<M: Memory> Machine<M> {
    fn eval_statement(&mut self, Statement::SetDiscriminant { destination, value }: Statement) -> NdResult {
        let (place, Type::Enum { variants, .. }) = self.eval_place(destination)? else {
            panic!("Setting the discriminant type of a non-enum contradicts well-formedness.");
        };
        if !place.aligned {
            throw_ub!("Setting the discriminant of a place based on a misaligned pointer");
        }

        let tagger = match variants.get(value) {
            Some(Variant { tagger, .. }) => tagger,
            // guaranteed unreachable by the well-formedness checks
            None => panic!("Setting an invalid discriminant ({value})"),
        };

        // Write the tag directly into memory.
        // This should be fine as we don't allow encoded data and the tag to overlap for valid enum variants.
        let accessor = |offset: Offset, bytes| {
            let ptr = self.ptr_offset_inbounds(place.ptr.thin_pointer, offset.bytes())?;
            // We have ensured that the place is aligned, so no alignment requirement here
            self.mem.store(ptr, bytes, Align::ONE, Atomicity::None)
        };
        encode_discriminant::<M>(accessor, tagger)?;
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
impl<M: Memory> ConcurrentMemory<M> {
    fn deinit(&mut self, ptr: ThinPointer<M::Provenance>, len: Size, align: Align) -> Result {
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
        self.mem.deinit(p.ptr.thin_pointer, ty.size::<M::T>(), Align::ONE)?;

        ret(())
    }
}
```

## StorageDead and StorageLive

These operations (de)allocate the memory backing a local.

```rust
impl<M: Memory> StackFrame<M> {
    fn storage_live(&mut self, mem: &mut ConcurrentMemory<M>, local: LocalName) -> NdResult {
        // First remove the old storage, if any.
        // This means the same address may be re-used for the new stoage.
        self.storage_dead(mem, local)?;
        // Then allocate the new storage.
        let layout = self.func.locals[local].layout::<M::T>();
        let ptr = mem.allocate(AllocationKind::Stack, layout.size, layout.align)?;
        self.locals.insert(local, ptr);
        ret(())
    }

    fn storage_dead(&mut self, mem: &mut ConcurrentMemory<M>, local: LocalName) -> NdResult {
        let layout = self.func.locals[local].layout::<M::T>();
        if let Some(ptr) = self.locals.remove(local) {
            mem.deallocate(ptr, AllocationKind::Stack, layout.size, layout.align)?;
        }
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
