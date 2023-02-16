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

    /// Stores a pointer to each of the global allocations.
    global_ptrs: Map<GlobalName, Pointer<M::Provenance>>,
}

/// The data that makes up a stack frame.
struct StackFrame<M: Memory> {
    /// The function this stack frame belongs to.
    func: Function,

    /// For each live local, the place in memory where its value is stored.
    locals: Map<LocalName, Place<M>>,

    /// Expresses what the caller does after the callee (this function) returns.
    caller_return_info: CallerReturnInfo<M>,

    /// `next_block` and `next_stmt` describe the next statement/terminator to execute (the "program counter").
    /// `next_block` identifies the basic block,
    next_block: BbName,

    /// If `next_stmt` is equal to the number of statements in this block (an
    /// out-of-bounds index in the statement list), it refers to the terminator.
    next_stmt: Int,
}

struct CallerReturnInfo<M: Memory> {
    /// The basic block to jump to when the callee returns.
    /// If `None`, UB will be raised when the callee returns.
    next_block: Option<BbName>,
    /// The place where the caller wants to see the return value,
    /// and the type it should be stored at.
    /// If `None`, the return value will be discarded.
    ret_place: Option<(Place<M>, PlaceType)>
}
```

Next, we define how to create a machine.

```rust
impl<M: Memory> Machine<M> {
    pub fn new(prog: Program) -> NdResult<Machine<M>> {
        if prog.check_wf::<M>().is_none() {
            throw_ill_formed!();
        }

        let mut mem = M::new();
        let mut global_ptrs = Map::new();

        // Allocate every global.
        for (global_name, global) in prog.globals {
            let size = Size::from_bytes(global.bytes.len()).unwrap();
            let alloc = mem.allocate(size, global.align)?;
            global_ptrs.insert(global_name, alloc);
        }

        // Fill the allocations.
        for (global_name, global) in prog.globals {
            let mut bytes = global.bytes.map(|b|
                match b {
                    Some(x) => AbstractByte::Init(x, None),
                    None => AbstractByte::Uninit
                }
            );
            for (i, relocation) in global.relocations {
                let ptr = global_ptrs[relocation.name].wrapping_offset::<M>(relocation.offset.bytes());
                let encoded_ptr = encode_ptr::<M>(ptr);
                bytes.write_subslice_at_index(i.bytes(), encoded_ptr);
            }
            mem.store(global_ptrs[global_name], bytes, global.align)?;
        }

        let start_fn = prog.functions[prog.start];

        // Setup the initial stack frame.
        // Well-formedness ensures that this has neither arguments nor a return local.
        let init_frame = StackFrame {
            func: start_fn,
            locals: Map::new(),
            caller_return_info: CallerReturnInfo {
                // The start function should never return.
                next_block: None,
                ret_place: None,
            },
            next_block: start_fn.start,
            next_stmt: Int::ZERO,
        };

        ret(Machine {
            prog,
            mem,
            intptrcast: IntPtrCast::new(),
            stack: list![init_frame],
            global_ptrs,
        })
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

        ret(())
    }
}
```
