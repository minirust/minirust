# Intrinsics

```rust
impl<M: Memory> Machine<M> {
    #[specr::argmatch(intrinsic)]
    fn eval_intrinsic(
        &mut self,
        intrinsic: Intrinsic,
        arguments: List<Value<M>>,
        ret_place: Option<Place<M>>,
    ) -> NdResult;
}
```

We start with the `Exit` intrinsic.

```rust
impl<M: Memory> Machine<M> {
    fn eval_intrinsic(
        &mut self,
        Intrinsic::Exit: Intrinsic,
        arguments: List<Value<M>>,
        ret_place: Option<Place<M>>,
    ) -> NdResult {
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
        ret_place: Option<Place<M>>,
    ) -> NdResult {
        self.eval_print(&mut std::io::stdout(), arguments, ret_place)?;

        ret(())
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::PrintStderr: Intrinsic,
        arguments: List<Value<M>>,
        ret_place: Option<Place<M>>,
    ) -> NdResult {
        self.eval_print(&mut std::io::stderr(), arguments, ret_place)?;

        ret(())
    }

    fn eval_print(
        &mut self,
        stream: &mut impl std::io::Write,
        arguments: List<Value<M>>,
        _ret_place: Option<Place<M>>,
    ) -> NdResult {
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

And finally the intrinsics used for memory allocation and deallocation.

```rust
impl<M: Memory> Machine<M> {
    fn eval_intrinsic(
        &mut self,
        Intrinsic::Allocate: Intrinsic,
        arguments: List<Value<M>>,
        ret_place: Option<Place<M>>,
    ) -> NdResult {
        let Some(ret_place) = ret_place else {
            throw_ub!("call to `Intrinsic::Allocate` is missing a return place");
        };

        if arguments.len() != 2 {
            throw_ub!("invalid number of arguments for `Intrinsic::Allocate`");
        }
        let Value::Int(size) = arguments[0] else {
            throw_ub!("invalid first argument to `Intrinsic::Allocate`");
        };
        let size = Size::from_bytes(size);

        let Value::Int(align) = arguments[1] else {
            throw_ub!("invalid second argument to `Intrinsic::Allocate`");
        };
        let Some(align) = Align::from_bytes(align) else {
            throw_ub!("invalid Align for `Intrinsic::Allocate`");
        };

        let alloc = self.mem.allocate(size, align)?;
        let bytes = encode_ptr::<M>(alloc);
        self.mem.store(ret_place, bytes, M::PTR_ALIGN)?;

        ret(())
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::Deallocate: Intrinsic,
        arguments: List<Value<M>>,
        ret_place: Option<Place<M>>,
    ) -> NdResult {
        if arguments.len() != 3 {
            throw_ub!("invalid number of arguments for `Intrinsic::Deallocate`");
        }

        let Value::Ptr(ptr) = arguments[0] else {
            throw_ub!("invalid first argument to `Intrinsic::Deallocate`");
        };

        let Value::Int(size) = arguments[1] else {
            throw_ub!("invalid second argument to `Intrinsic::Deallocate`");
        };
        let size = Size::from_bytes(size);

        let Value::Int(align) = arguments[2] else {
            throw_ub!("invalid third argument to `Intrinsic::Deallocate`");
        };
        let Some(align) = Align::from_bytes(align) else {
            throw_ub!("invalid Align for `Intrinsic::Deallocate`");
        };

        self.mem.deallocate(ptr, size, align)?;

        ret(())
    }
}
```
