# Intrinsics

```rust
impl<M: Memory> Machine<M> {
    #[specr::argmatch(intrinsic)]
    fn eval_intrinsic(
        &mut self,
        intrinsic: Intrinsic,
        arguments: List<Value<M>>,
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

fn is_unit(ty: Type) -> bool {
    let Type::Tuple{size, fields} = ty else {
        return false;
    };

    size == Size::ZERO && fields.is_empty()
} 
```

We start with the `Exit` intrinsic.

```rust
impl<M: Memory> Machine<M> {
    fn eval_intrinsic(
        &mut self,
        Intrinsic::Exit: Intrinsic,
        arguments: List<Value<M>>,
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
        arguments: List<Value<M>>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if !is_unit(ret_ty) {
            throw_ub!("invalid return type for `Intrinsic::PrintStdout`")
        }

        self.eval_print(&mut std::io::stdout(), arguments)?;

        ret(unit_value())
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::PrintStderr: Intrinsic,
        arguments: List<Value<M>>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if !is_unit(ret_ty) {
            throw_ub!("invalid return type for `Intrinsic::PrintStderr`")
        }

        self.eval_print(&mut std::io::stderr(), arguments)?;

        ret(unit_value())
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
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
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

        if !matches!(ret_ty, Type::Ptr(_)) {
            throw_ub!("invalid return type for `Intrinsic::Allocate`")
        }

        let alloc = self.mem.allocate(size, align)?;

        ret(Value::Ptr(alloc))
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::Deallocate: Intrinsic,
        arguments: List<Value<M>>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
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

        if !is_unit(ret_ty) {
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
    fn eval_intrinsic(
        &mut self,
        Intrinsic::Spawn: Intrinsic,
        arguments: List<Value<M>>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
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

        if !matches!(ret_ty, Type::Int(_)) {
            throw_ub!("invalid return type for `Intrinsic::Spawn`")
        }

        let thread_id = self.thread_manager.spawn(func)?;

        ret(Value::Int(thread_id))
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::Join: Intrinsic,
        arguments: List<Value<M>>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if arguments.len() != 1 {
            throw_ub!("invalid number of arguments for `Intrinsic::Join`");
        }

        let Value::Int(thread_id) = arguments[0] else {
            throw_ub!("invalid first argument to `Intrinsic::Join`");
        };

        if !is_unit(ret_ty) {
            throw_ub!("invalid return type for `Intrinsic::Join`")
        }

        self.thread_manager.join(thread_id)?;

        ret(unit_value())
    }
}
```
