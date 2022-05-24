# MiniRust Abstract Machine

This defines the state that makes up the MiniRust Abstract Machine:
which components together make up the state of a MiniRust program during its execution?
This key data structure says a lot about how the Abstract Machine is structured.
(The "reduction relation" aka operational semantics aka `step` function is defined in [the next file](step.md).)

```rust
/// This type contains everything that needs to be tracked during the execution
/// of a MiniRust program.
struct Machine {
    /// The program we are executing.
    prog: Program,

    /// The state of memory.
    mem: Memory,

    /// The stack.
    stack: List<StackFrame>,
}

/// The data that makes up a stack frame.
struct StackFrame {
    /// The function this stack frame belongs to.
    func: Function,

    /// For each live local, the place in memory where its value is stored.
    locals: Map<LocalName, Place>,

    /// The next statement/terminator to execute (the "program counter").
    /// The first component identifies the basic block,
    /// the second the statement inside that basic block.
    /// If the index is len+1, it refers to the terminator.
    next: (BbName, u64),
}
```

We also define a bunch of helper functions that will be useful later.

```rust
impl Machine {
    fn cur_frame(&self) -> &StackFrame {
        self.stack.last_mut().unwrap()
    }

    fn cur_frame_mut(&mut self) -> &mut StackFrame {
        self.stack.last_mut().unwrap()
    }
}
```
