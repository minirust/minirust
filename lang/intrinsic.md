# Intrinsics

```rust
pub enum Intrinsic {
    Exit,
    PrintStdout,
    PrintStderr,
}

impl<M: Memory> Machine<M> {
    fn eval_intrinsic(
        &mut self,
        intrinsic: Intrinsic,
        arguments: List<Value<M>>,
        ret: Option<Place<M>>,
        next_block: Option<BbName>,
    ) -> NdResult {
        match intrinsic {
            Intrinsic::Exit => throw_machine_stop!("0"),
            Intrinsic::PrintStdout => self.eval_print(&mut std::io::stdout(), arguments, ret, next_block)?,
            Intrinsic::PrintStderr => self.eval_print(&mut std::io::stderr(), arguments, ret, next_block)?,
        }
    }

    fn eval_print(
        &mut self,
        stream: &mut impl std::io::Write,
        arguments: List<Value<M>>,
        _ret: Option<Place<M>>,
        next_block: Option<BbName>
    ) -> NdResult {
        let Some(next_block) = next_block else {
            throw_ub!("missing next_block for print intrinsic");
        };

        for arg in arguments {
            match arg {
                Value::Int(i) => write!(stream, "{}\n", i).unwrap(),
                Value::Bool(b) => write!(stream, "{}\n", b).unwrap(),
                _ => throw_ub!("unsupported value for printing"),
            }
        }

        self.mutate_cur_frame(|frame| {
            frame.jump_to_block(next_block);
        });
    }
}
```
