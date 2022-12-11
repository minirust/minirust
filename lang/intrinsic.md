# Intrinsics

```rust
impl<M: Memory> Machine<M> {
    #[specr::argmatch(intrinsic)]
    fn eval_intrinsic(
        &mut self,
        intrinsic: Intrinsic,
        arguments: List<Value<M>>,
        ret: Option<Place<M>>,
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
        ret: Option<Place<M>>,
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
        ret: Option<Place<M>>,
    ) -> NdResult {
        self.eval_print(&mut std::io::stdout(), arguments, ret)?
    }

    fn eval_intrinsic(
        &mut self,
        Intrinsic::PrintStderr: Intrinsic,
        arguments: List<Value<M>>,
        ret: Option<Place<M>>,
    ) -> NdResult {
        self.eval_print(&mut std::io::stderr(), arguments, ret)?
    }

    fn eval_print(
        &mut self,
        stream: &mut impl std::io::Write,
        arguments: List<Value<M>>,
        _ret: Option<Place<M>>,
    ) -> NdResult {
        for arg in arguments {
            match arg {
                Value::Int(i) => write!(stream, "{}\n", i).unwrap(),
                Value::Bool(b) => write!(stream, "{}\n", b).unwrap(),
                _ => throw_ub!("unsupported value for printing"),
            }
        }
    }
}
```
