# MiniRust Abstract Machine

This defines the state that makes up the MiniRust Abstract Machine:
which components together make up the state of a MiniRust program during its execution?
This key data structure says a lot about how the Abstract Machine is structured.
(The "reduction relation" aka operational semantics aka `step` function is defined in [the next file](step.md).)

```rust
/// This type contains everything that needs to be tracked during the execution
/// of a MiniRust program.
pub struct Machine<M: Memory> {
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

    /// `next_block` and `next_stmt` describe the next statement/terminator to execute (the "program counter").
    /// `next_block` identifies the basic block,
    next_block: BbName,

    /// If `next_stmt` is equal to the number of statements in this block (an
    /// out-of-bounds index in the statement list), it refers to the terminator.
    next_stmt: Int,
}
```

Next, we define the functions necessary to create and run a machine.

```rust
impl<M: Memory> Machine<M> {
    pub fn new(prog: Program) -> Machine<M> {
        let start_fn = prog.functions[prog.start];

        let null_ptr = Pointer {
            addr: Int::ZERO,
            provenance: None
        };

        // Setup the initial stack frame.
        // Well-formedness ensures that this has neither arguments nor a return local.
        let init_frame = StackFrame {
            func: start_fn,
            locals: Map::new(),
            // Without a return local, `caller_ret_place` does not matter.
            caller_ret_place: null_ptr,
            next_block: start_fn.start,
            next_stmt: Int::ZERO,
        };

        Machine {
            prog,
            mem: M::new(),
            intptrcast: IntPtrCast::new(),
            stack: list![init_frame],
        }
    }

    pub fn run(&mut self) -> NdResult<!> {
        loop {
            self.step()?;
        }
    }
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

impl<M: Memory> StackFrame<M> {
    /// jump to the beginning of the given block.
    fn jump_to_block(&mut self, b: BbName) -> NdResult {
        self.next_block = b;
        self.next_stmt = Int::ZERO;
    }
}
```
