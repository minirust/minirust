# Intrinsics

```rust
impl<M: Memory> Machine<M> {
    #[specr::argmatch(intrinsic)]
    fn eval_intrinsic(
        &mut self,
        intrinsic: Intrinsic,
        arguments: List<Value<M>>,
    ) -> NdResult<(Value<M>, Type)> { .. }
}
```

This helper functions simplifies unit-returning intrinsics.

```rust
fn unit<M: Memory>() -> (Value<M>, Type) {
    (Value::Tuple(list![]), Type::Tuple{fields:list![], size: Size::ZERO})
}
```

We start with the `Exit` intrinsic.

```rust
impl<M: Memory> Machine<M> {
    fn eval_intrinsic(
        &mut self,
        Intrinsic::Exit: Intrinsic,
        arguments: List<Value<M>>,
    ) -> NdResult<(Value<M>, Type)> {
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
        arguments: List<Value<M>>,
    ) -> NdResult<(Value<M>, Type)> {
        self.eval_print(&mut std::io::stdout(), arguments)?;

        ret(unit())
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::PrintStderr: Intrinsic,
        arguments: List<Value<M>>,
    ) -> NdResult<(Value<M>, Type)> {
        self.eval_print(&mut std::io::stderr(), arguments)?;

        ret(unit())
    }

    fn eval_print(
        &mut self,
        stream: &mut impl std::io::Write,
        arguments: List<Value<M>>,
    ) -> Result {
        for arg in arguments {
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
        arguments: List<Value<M>>,
    ) -> NdResult<(Value<M>, Type)> {
        if arguments.len() != 2 {
            throw_ub!("invalid number of arguments for `Intrinsic::Allocate`");
        }
        let Value::Int(size) = arguments[0] else {
            throw_ub!("invalid first argument to `Intrinsic::Allocate`");
        };
        let Some(size) = Size::from_bytes(size) else {
            throw_ub!("invalid size for `Intrinsic::Allocate`: negative size");
        };

        let Value::Int(align) = arguments[1] else {
            throw_ub!("invalid second argument to `Intrinsic::Allocate`");
        };
        let Some(align) = Align::from_bytes(align) else {
            throw_ub!("invalid alignment for `Intrinsic::Allocate`: not a power of 2");
        };

        let alloc = self.mem.allocate(size, align)?;

        // `Allocate` returns `*mut ()`.
        let pointee = Layout {
            size: Size::ZERO,
            align: Align::ONE,
            inhabited: true,
        };

        ret((Value::Ptr(alloc), Type::Ptr(PtrType::Raw{pointee})))
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::Deallocate: Intrinsic,
        arguments: List<Value<M>>,
    ) -> NdResult<(Value<M>, Type)> {
        if arguments.len() != 3 {
            throw_ub!("invalid number of arguments for `Intrinsic::Deallocate`");
        }

        let Value::Ptr(ptr) = arguments[0] else {
            throw_ub!("invalid first argument to `Intrinsic::Deallocate`");
        };

        let Value::Int(size) = arguments[1] else {
            throw_ub!("invalid second argument to `Intrinsic::Deallocate`");
        };
        let Some(size) = Size::from_bytes(size) else {
            throw_ub!("invalid size for `Intrinsic::Deallocate`: negative size");
        };

        let Value::Int(align) = arguments[2] else {
            throw_ub!("invalid third argument to `Intrinsic::Deallocate`");
        };
        let Some(align) = Align::from_bytes(align) else {
            throw_ub!("invalid alignment for `Intrinsic::Deallocate`: not a power of 2");
        };

        self.mem.deallocate(ptr, size, align)?;

        ret(unit())
    }
}
```

The intrinsics for spawning and joining threads.

```rust
impl<M: Memory> Machine<M> {
    fn eval_intrinsic(
        &mut self,
        Intrinsic::Spawn: Intrinsic,
        arguments: List<Value<M>>,
    ) -> NdResult<(Value<M>, Type)> {
        if arguments.len() != 1 {
            throw_ub!("invalid number of arguments for `Intrinsic::Spawn`");
        }

        let Value::Ptr(ptr) = arguments[0] else {
            throw_ub!("invalid first argument to `Intrinsic::Spawn`");
        };

        let func = self.fn_from_addr(ptr.addr)?;

        if func.args.len() != 0 {
            throw_ub!("invalid first argument to `Intrinsic::Spawn`, function takes arguments");
        }

        // This is taken from Miri. It also does not allow for a return value in the root function of a thread.
        if func.ret.is_some() {
            throw_ub!("invalid first argument to `Intrinsic::Spawn`, function returns something");
        }

        // FIXME: What if the thread_id doesn't fit into a u32?
        let thread_id = self.thread_manager.spawn(func)?;

        let id_type = Type::Int(IntType{
            signed: Signedness::Unsigned,
            size: Size::from_bits(Int::from(32)).unwrap(),
        });

        ret((Value::Int(thread_id), id_type))
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::Join: Intrinsic,
        arguments: List<Value<M>>,
    ) -> NdResult<(Value<M>, Type)> {
        if arguments.len() != 1 {
            throw_ub!("invalid number of arguments for `Intrinsic::Join`");
        }

        let Value::Int(thread_id) = arguments[0] else {
            throw_ub!("invalid first argument to `Intrinsic::Join`");
        };

        self.thread_manager.join(thread_id)?;

        ret(unit())
    }
}
```
