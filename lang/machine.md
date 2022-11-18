# MiniRust Abstract Machine

This defines the state that makes up the MiniRust Abstract Machine:
which components together make up the state of a MiniRust program during its execution?
This key data structure says a lot about how the Abstract Machine is structured.
(The "reduction relation" aka operational semantics aka `step` function is defined in [the next file](step.md).)

```rust
/// This type contains everything that needs to be tracked during the execution
/// of a MiniRust program.
struct Machine<M: Memory> {
    /// The program we are executing.
    prog: Program,

    /// The state of memory.
    mem: M,

    /// The state of the integer-pointer cast subsystem.
    intptrcast: IntPtrCast<M::Provenance>,

    /// The stack.
    stack: List<StackFrame<M>>,
}

/// The data that makes up a stack frame.
struct StackFrame<M: Memory> {
    /// The function this stack frame belongs to.
    func: Function,

    /// For each live local, the place in memory where its value is stored.
    locals: Map<LocalName, Place<M>>,

    /// The place where the caller wants to see the return value.
    caller_ret_place: Place<M>,

    /// The next statement/terminator to execute (the "program counter").
    /// The first component identifies the basic block,
    /// the second the statement inside that basic block.
    /// If the index is len+1, it refers to the terminator.
    next: (BbName, BigInt),
}
```

We also define some helper functions that will be useful later.

```rust
impl<M: Memory> Machine<M> {
    fn cur_frame(&self) -> StackFrame<M> {
        self.stack.last().unwrap()
    }

    fn mutate_cur_frame<O>(&mut self, f: impl FnOnce(&mut StackFrame<M>) -> O) -> O {
        if self.stack.is_empty() {
            panic!("`mutate_cur_frame` called on empty stack!");
        }

        let last_idx = self.stack.len() - 1;
        self.stack.mutate_at(last_idx, f)
    }
}
```
