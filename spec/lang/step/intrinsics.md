# Intrinsics

This file defines the generic machine intrinsics.

```rust
impl<M: Memory> Machine<M> {
    #[specr::argmatch(intrinsic)]
    fn eval_intrinsic(
        &mut self,
        intrinsic: Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> { .. }
}
```

These helper functions simplify unit-returning intrinsics.

```rust
fn unit_value<M: Memory>() -> Value<M> {
    Value::Tuple(list![])
}

fn unit_type() -> Type {
    Type::Tuple { fields: list![], size: Size::ZERO, align: Align::ONE }
}
```

## Machine primitives

We start with the `Exit` intrinsic.

```rust
impl<M: Memory> Machine<M> {
    fn exit(&self) -> NdResult<!> {
        // Check for memory leaks.
        self.mem.leak_check()?;
        // No leak found -- good, stop the machine.
        throw_machine_stop!();
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::Exit: Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        self.exit()?
    }
}
```

## Input and output

These are the `PrintStdout` and `PrintStderr` intrinsics.

```rust
impl<M: Memory> Machine<M> {
    fn eval_intrinsic(
        &mut self,
        Intrinsic::PrintStdout: Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if ret_ty != unit_type() {
            throw_ub!("invalid return type for `Intrinsic::PrintStdout`")
        }

        self.eval_print(self.stdout, arguments)?;

        ret(unit_value())
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::PrintStderr: Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if ret_ty != unit_type() {
            throw_ub!("invalid return type for `Intrinsic::PrintStderr`")
        }

        self.eval_print(self.stderr, arguments)?;

        ret(unit_value())
    }

    fn eval_print(
        &mut self,
        stream: DynWrite,
        arguments: List<(Value<M>, Type)>,
    ) -> Result {
        for (arg, _) in arguments {
            match arg {
                Value::Int(i) => write!(stream, "{}\n", i).unwrap(),
                Value::Bool(b) => write!(stream, "{}\n", b).unwrap(),
                _ => throw_ub!("unsupported value for printing"),
            }
        }

        ret(())
    }
}
```

## Heap memory management

These intrinsics can be used for dynamic memory allocation and deallocation.

```rust
impl<M: Memory> Machine<M> {
    fn eval_intrinsic(
        &mut self,
        Intrinsic::Allocate: Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if arguments.len() != 2 {
            throw_ub!("invalid number of arguments for `Intrinsic::Allocate`");
        }

        let Value::Int(size) = arguments[0].0 else {
            throw_ub!("invalid first argument to `Intrinsic::Allocate`, not an integer");
        };
        let Some(size) = Size::from_bytes(size) else {
            throw_ub!("invalid size for `Intrinsic::Allocate`: negative size");
        };

        let Value::Int(align) = arguments[1].0 else {
            throw_ub!("invalid second argument to `Intrinsic::Allocate`, not an integer");
        };
        let Some(align) = Align::from_bytes(align) else {
            throw_ub!("invalid alignment for `Intrinsic::Allocate`: not a power of 2");
        };

        if !matches!(ret_ty, Type::Ptr(_)) {
            throw_ub!("invalid return type for `Intrinsic::Allocate`")
        }

        let alloc = self.mem.allocate(AllocationKind::Heap, size, align)?;

        ret(Value::Ptr(alloc))
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::Deallocate: Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if arguments.len() != 3 {
            throw_ub!("invalid number of arguments for `Intrinsic::Deallocate`");
        }

        let Value::Ptr(ptr) = arguments[0].0 else {
            throw_ub!("invalid first argument to `Intrinsic::Deallocate`, not a pointer");
        };

        let Value::Int(size) = arguments[1].0 else {
            throw_ub!("invalid second argument to `Intrinsic::Deallocate`, not an integer");
        };
        let Some(size) = Size::from_bytes(size) else {
            throw_ub!("invalid size for `Intrinsic::Deallocate`: negative size");
        };

        let Value::Int(align) = arguments[2].0 else {
            throw_ub!("invalid third argument to `Intrinsic::Deallocate`, not an integer");
        };
        let Some(align) = Align::from_bytes(align) else {
            throw_ub!("invalid alignment for `Intrinsic::Deallocate`: not a power of 2");
        };

        if ret_ty != unit_type() {
            throw_ub!("invalid return type for `Intrinsic::Deallocate`")
        }

        self.mem.deallocate(ptr, AllocationKind::Heap, size, align)?;

        ret(unit_value())
    }
}
```

## Threads

These intrinsics let the program spawn and join threads.

```rust
impl<M: Memory> Machine<M> {
    fn spawn(&mut self, func: Function, data_pointer: Value<M>, data_ptr_ty: Type) -> NdResult<ThreadId> {
        // Create the thread.
        let args = list![(data_pointer, data_ptr_ty)];
        let thread_id = self.new_thread(func, args)?;

        // This thread got synchronized because its existence startet with this.
        self.synchronized_threads.insert(thread_id);

        ret(thread_id)
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::Spawn: Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if arguments.len() != 2 {
            throw_ub!("invalid number of arguments for `Intrinsic::Spawn`");
        }

        let Value::Ptr(ptr) = arguments[0].0 else {
            throw_ub!("invalid first argument to `Intrinsic::Spawn`, not a pointer");
        };
        let func = self.fn_from_addr(ptr.addr)?;

        let (data_ptr, data_ptr_ty) = arguments[1];
        if !matches!(data_ptr_ty, Type::Ptr(_)) {
            throw_ub!("invalid second argument to `Intrinsic::Spawn`, not a pointer");
        }

        if !matches!(ret_ty, Type::Int(_)) {
            throw_ub!("invalid return type for `Intrinsic::Spawn`")
        }

        let thread_id = self.spawn(func, data_ptr, data_ptr_ty)?;
        ret(Value::Int(thread_id))
    }

    fn join(&mut self, thread_id: ThreadId) -> NdResult {
        let Some(thread) = self.threads.get(thread_id) else {
            throw_ub!("`Intrinsic::Join`: join non existing thread");
        };

        match thread.state {
            ThreadState::Terminated => {},
            _ => {
                self.threads.mutate_at(self.active_thread, |thread|{
                    thread.state = ThreadState::BlockedOnJoin(thread_id);
                });
            },
        };

        ret(())
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::Join: Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if arguments.len() != 1 {
            throw_ub!("invalid number of arguments for `Intrinsic::Join`");
        }

        let Value::Int(thread_id) = arguments[0].0 else {
            throw_ub!("invalid first argument to `Intrinsic::Join`, not an integer");
        };

        if ret_ty != unit_type() {
            throw_ub!("invalid return type for `Intrinsic::Join`")
        }

        self.join(thread_id)?;
        ret(unit_value())
    }
}
```

## Atomic accesses

These intrinsics provide atomic accesses.

```rust
impl<M: Memory> Machine<M> {
    fn eval_intrinsic(
        &mut self,
        Intrinsic::AtomicStore: Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if arguments.len() != 2 {
            throw_ub!("invalid number of arguments for `Intrinsic::AtomicStore`");
        }

        let Value::Ptr(ptr) = arguments[0].0 else {
            throw_ub!("invalid first argument to `Intrinsic::AtomicStore`, not a pointer");
        };

        let (val, ty) = arguments[1];
        let size = ty.size::<M::T>();
        let Some(align) = Align::from_bytes(size.bytes()) else {
            throw_ub!("invalid second argument to `Intrinsic::AtomicStore`, size not power of two");
        };
        if size > M::T::MAX_ATOMIC_SIZE {
            throw_ub!("invalid second argument to `Intrinsic::AtomicStore`, size too big");
        }

        if ret_ty != unit_type() {
            throw_ub!("invalid return type for `Intrinsic::AtomicStore`")
        }

        self.mem.typed_store(ptr, val, ty, align, Atomicity::Atomic)?;
        ret(unit_value())
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::AtomicLoad: Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if arguments.len() != 1 {
            throw_ub!("invalid number of arguments for `Intrinsic::AtomicLoad`");
        }
    
        let Value::Ptr(ptr) = arguments[0].0 else {
            throw_ub!("invalid first argument to `Intrinsic::AtomicLoad`, not a pointer");
        };

        let size = ret_ty.size::<M::T>();
        let Some(align) = Align::from_bytes(size.bytes()) else {
            throw_ub!("invalid return type for `Intrinsic::AtomicLoad`, size not power of two");
        };
        if size > M::T::MAX_ATOMIC_SIZE {
            throw_ub!("invalid return type for `Intrinsic::AtomicLoad`, size too big");
        }

        let val = self.mem.typed_load(ptr, ret_ty, align, Atomicity::Atomic)?;
        ret(val)
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::AtomicCompareExchange: Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if arguments.len() != 3 {
            throw_ub!("invalid number of arguments for `Intrinsic::AtomicCompareExchange`");
        }

        let Value::Ptr(ptr) = arguments[0].0 else {
            throw_ub!("invalid first argument to `Intrinsic::AtomicCompareExchange`, not a pointer");
        };

        let (current, curr_ty) = arguments[1];
        if curr_ty != ret_ty {
            throw_ub!("invalid second argument to `Intrinsic::AtomicCompareExchange`, not same type as return value");
        }

        let (next, next_ty) = arguments[2];
        if next_ty != ret_ty {
            throw_ub!("invalid third argument to `Intrinsic::AtomicCompareExchange`, not same type as return value");
        }

        if !matches!(ret_ty, Type::Int(_)) {
            throw_ub!("invalid return type for `Intrinis::AtomicCompareExchange`, only works with integers");
        }

        let size = ret_ty.size::<M::T>();
        // All integer sizes are powers of two.
        let align = Align::from_bytes(size.bytes()).unwrap();
        if size > M::T::MAX_ATOMIC_SIZE {
            throw_ub!("invalid return type for `Intrinsic::AtomicCompareExchange`, size to big");
        }

        // The value at the location right now.
        let before = self.mem.typed_load(ptr, ret_ty, align, Atomicity::Atomic)?;

        // This is the central part of the operation. If the expected before value at ptr is the current value,
        // then we exchange it for the next value.
        // FIXME: The memory model might have to know that this is a compare-exchange.
        if current == before {
            self.mem.typed_store(ptr, next, ret_ty, align, Atomicity::Atomic)?;
        } else {
            // We do *not* do a store on a failing AtomicCompareExchange. This means that races between
            // a non-atomic load and a failing AtomicCompareExchange are not considered UB!
        }

        ret(before)
    }
}
```

We also implement the atomic fetch operations. First we define a helper function to decide which operations can be in a fetch operation.

```rust
/// Predicate to indicate if integer bin-op can be used for atomic fetch operations.
fn is_atomic_binop(op: BinOpInt) -> bool {
    use BinOpInt as B;
    match op {
        B::Add | B::Sub => true,
        _ => false
    }
}

impl<M: Memory> Machine<M> {
    fn eval_intrinsic(
        &mut self,
        Intrinsic::AtomicFetch(op): Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if arguments.len() != 2 {
            throw_ub!("invalid number of arguments for `Intrinsic::AtomicFetch`");
        }

        let Value::Ptr(ptr) = arguments[0].0 else {
            throw_ub!("invalid first argument to `Intrinsic::AtomicFetch`, not a pointer");
        };

        let (other, other_ty) = arguments[1];
        if other_ty != ret_ty {
            throw_ub!("invalid second argument to `Intrinsic::AtomicFetch`, not same type as return value");
        }

        if !matches!(ret_ty, Type::Int(_)) {
            throw_ub!("invalid return type for `Intrinis::AtomicFetch`, only works with integers");
        }

        let size = ret_ty.size::<M::T>();
        // All integer sizes are powers of two.
        let align = Align::from_bytes(size.bytes()).unwrap();
        if size > M::T::MAX_ATOMIC_SIZE {
            throw_ub!("invalid return type for `Intrinsic::AtomicFetch`, size to big");
        }

        if !is_atomic_binop(op) {
            throw_ub!("invalid bin op for `Intrinsic::AtomicFetch`");
        }

        // The value at the location right now.
        let previous = self.mem.typed_load(ptr, ret_ty, align, Atomicity::Atomic)?;

        // Convert to integers
        let Value::Int(other_int) = other else { unreachable!() };
        let Value::Int(previous_int) = previous else { unreachable!() };

        // Perform operation.
        let next_int = self.eval_bin_op_int(op, previous_int, other_int)?;
        let next = Value::Int(next_int);

        // Store it again.
        self.mem.typed_store(ptr, next, ret_ty, align, Atomicity::Atomic)?;

        ret(previous)
    }
}
```
