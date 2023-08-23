# Intrinsics

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

This helper functions simplifies unit-returning intrinsics.

```rust
fn unit_value<M: Memory>() -> Value<M> {
    Value::Tuple(list![])
}

fn unit_type() -> Type {
    Type::Tuple { fields: list![], size: Size::ZERO }
}

fn unit_ptype() -> PlaceType {
    PlaceType { ty: unit_type(), align: Align::ONE }
}
```

We start with the `Exit` intrinsic.

```rust
impl<M: Memory> Machine<M> {
    fn eval_intrinsic(
        &mut self,
        Intrinsic::Exit: Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        throw_machine_stop!()
    }
}
```

And there are the `PrintStdout` and `PrintStderr` intrinsics.

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

Next, the intrinsics used for memory allocation and deallocation.

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

        let alloc = self.mem.allocate(size, align)?;

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

        self.mem.deallocate(ptr, size, align)?;

        ret(unit_value())
    }
}
```

The intrinsics for spawning and joining threads.

```rust
impl<M: Memory> Machine<M> {
    fn spawn(&mut self, func: Function, data_pointer: Value<M>, data_ptr_ty: Type) -> NdResult<ThreadId> {
        let thread_id = ThreadId::from(self.threads.len());
        let frame = self.create_frame(func, ReturnAction::BottomOfStack)?;
        // TODO: there is probably a nice helper function that could be factored to shared more code with `Call`.

        // FIXME: check callee ABI

        // Check return type ABI compatibility.
        if let Some(ret_local) = func.ret {
            if !check_abi_compatibility(unit_ptype(), func.locals[ret_local]) {
                throw_ub!("spawned threads must have return type that is ABI-compatible with `()`");
            }
        } else {
            // No return local in the callee, nothing can go wrong.
        }

        // Pass the data pointer and make sure the callee argument is ABI-compatible.
        if func.args.len() != 1 {
            throw_ub!("spawned threads must take exactly one argument");
        }
        let arg_local = func.args[0];
        let data_ptr_pty = PlaceType::new(data_ptr_ty, M::T::PTR_ALIGN);
        if !check_abi_compatibility(data_ptr_pty, func.locals[arg_local]) {
            throw_ub!("spawned thread must take a pointer as first argument");
        }
        self.mem.typed_store(frame.locals[arg_local], data_pointer, data_ptr_pty, Atomicity::None)?;

        // Done! We can create the thread.
        self.threads.push(Thread::new(frame));

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

These are the intrinsics for atomic memory accesses:

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
        if !size.bytes().is_power_of_two() {
            throw_ub!("invalid second argument to `Intrinsic::AtomicStore`, size not power of two");
        }
        if size > M::T::MAX_ATOMIC_SIZE {
            throw_ub!("invalid second argument to `Intrinsic::AtomicStore`, size too big");
        }

        if ret_ty != unit_type() {
            throw_ub!("invalid return type for `Intrinsic::AtomicStore`")
        }

        let pty = PlaceType { ty, align: Align::from_bytes(size.bytes()).unwrap() };
        self.mem.typed_store(ptr, val, pty, Atomicity::Atomic)?;
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
        if !size.bytes().is_power_of_two() {
            throw_ub!("invalid return type for `Intrinsic::AtomicLoad`, size not power of two");
        }
        if size > M::T::MAX_ATOMIC_SIZE {
            throw_ub!("invalid return type for `Intrinsic::AtomicLoad`, size too big");
        }

        let pty = PlaceType { ty: ret_ty, align: Align::from_bytes(size.bytes()).unwrap() };
        let val = self.mem.typed_load(ptr, pty, Atomicity::Atomic)?;
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
        assert!(size.bytes().is_power_of_two());
        if size > M::T::MAX_ATOMIC_SIZE {
            throw_ub!("invalid return type for `Intrinsic::AtomicCompareExchange`, size to big");
        }
        
        let pty = PlaceType { ty: ret_ty, align: Align::from_bytes(size.bytes()).unwrap() };

        // The value at the location right now.
        let before = self.mem.typed_load(ptr, pty, Atomicity::Atomic)?;

        // This is the central part of the operation. If the expected before value at ptr is the current value,
        // then we exchange it for the next value.
        // FIXME: The memory model might have to know that this is a compare-exchange.
        if current == before {
            self.mem.typed_store(ptr, next, pty, Atomicity::Atomic)?;
        } else {
            // We do *not* do a store on a failing AtomicCompareExchange. This means that races between
            // a non-atomic read and a failing AtomicCompareExchange are not considered UB!
        }

        ret(before)
    }
}
```
