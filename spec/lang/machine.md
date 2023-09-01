# MiniRust Abstract Machine

This defines the state that makes up the MiniRust Abstract Machine:
which components together make up the state of a MiniRust program during its execution?
This key data structure says a lot about how the Abstract Machine is structured.
(The "reduction relation" aka operational semantics aka `step` function is defined in [the next file](step.md).)

```rust
/// This type contains everything that needs to be tracked during the execution
/// of a MiniRust program.
#[no_obj]
pub struct Machine<M: Memory> {
    /// The program we are executing.
    prog: Program,

    /// The state of memory.
    mem: AtomicMemory<M>,

    /// The state of the integer-pointer cast subsystem.
    intptrcast: IntPtrCast<M::Provenance>,

    /// The Threads
    threads: List<Thread<M>>,

    /// The currently / most recently active thread.
    active_thread: ThreadId,

    /// A set of threads that have been synchronized.
    /// A thread being added here in a given step means that if the very next step is
    /// by that thread, we do *not* do data race detection: there was synchronization
    /// between these two steps, so any potential accesses are not racing.
    synchronized_threads: Set<ThreadId>,

    /// The Locks
    locks: List<LockState>,

    /// Stores a pointer to each of the global allocations.
    global_ptrs: Map<GlobalName, Pointer<M::Provenance>>,

    /// Stores an address for each function name.
    fn_addrs: Map<FnName, mem::Address>,

    /// This is where the `PrintStdout` intrinsic writes to.
    stdout: DynWrite,
    /// This is where the `PrintStderr` intrinsic writes to.
    stderr: DynWrite,
}

/// The data that makes up a stack frame.
struct StackFrame<M: Memory> {
    /// The function this stack frame belongs to.
    func: Function,

    /// For each live local, the place in memory where its value is stored.
    locals: Map<LocalName, Place<M>>,

    /// Expresses what happens after the callee (this function) returns.
    return_action: ReturnAction<M>,

    /// `next_block` and `next_stmt` describe the next statement/terminator to execute (the "program counter").
    /// `next_block` identifies the basic block,
    next_block: BbName,

    /// If `next_stmt` is equal to the number of statements in this block (an
    /// out-of-bounds index in the statement list), it refers to the terminator.
    next_stmt: Int,
}

enum ReturnAction<M: Memory> {
    /// This is the bottom of the stack, there is nothing left to do in this thread.
    BottomOfStack,
    /// Return to the caller.
    ReturnToCaller {
        /// The basic block to jump to when the callee returns.
        /// If `None`, UB will be raised when the callee returns.
        next_block: Option<BbName>,
        /// The location where the caller wants to see the return value.
        /// Has already been checked to be suitably compatible with the callee return type.
        ret_place: Place<M>,
    }
}
```

This defines the internal representation of a thread of execution.

```rust
pub struct Thread<M: Memory> {
    state: ThreadState,

    /// The stack.
    stack: List<StackFrame<M>>,
}

pub enum ThreadState {
    /// The thread is enabled and can get executed.
    Enabled,
    /// The thread is trying to join another thread and is blocked until that thread finishes.
    BlockedOnJoin(ThreadId),
    /// The thread is waiting to acquire a lock.
    BlockedOnLock(LockId),
    /// The thread has terminated.
    Terminated,
}
```

Next, we define how to create a machine.

```rust
impl<M: Memory> Machine<M> {
    pub fn new(prog: Program, stdout: DynWrite, stderr: DynWrite) -> NdResult<Machine<M>> {
        if prog.check_wf::<M::T>().is_none() {
            throw_ill_formed!();
        }

        let mut mem = AtomicMemory::<M>::new();
        let mut global_ptrs = Map::new();
        let mut fn_addrs = Map::new();

        // Allocate every global.
        for (global_name, global) in prog.globals {
            let size = Size::from_bytes(global.bytes.len()).unwrap();
            let alloc = mem.allocate(AllocationKind::Global, size, global.align)?;
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
            // This cannot fail, we just allocated that memory above.
            mem.store(global_ptrs[global_name], bytes, global.align, Atomicity::None).unwrap();
        }

        // Allocate functions.
        for (fn_name, _function) in prog.functions {
            let alloc = mem.allocate(AllocationKind::Function, Size::ZERO, Align::ONE)?;
            let addr = alloc.addr;
            // Ensure that no two functions lie on the same address.
            assert!(!fn_addrs.values().any(|fn_addr| addr == fn_addr));
            fn_addrs.insert(fn_name, addr);
        }

        // Create machine, without a thread yet.
        let mut machine = Machine {
            prog,
            mem,
            intptrcast: IntPtrCast::new(),
            global_ptrs,
            fn_addrs,
            threads: list![],
            locks: List::new(),
            active_thread: ThreadId::ZERO,
            synchronized_threads: Set::new(),
            stdout,
            stderr,
        };

        // Create initial thread.
        let start_fn = prog.functions[prog.start];
        machine.new_thread(start_fn, list![])?;

        ret(machine)
    }

    fn exit(&self) -> NdResult<!> {
        // Check for memory leaks.
        self.mem.leak_check()?;
        // No leak found -- good, stop the machine.
        throw_machine_stop!();
    }
}
```

We also define some helper functions that will be useful later.

```rust
impl<M: Memory> Machine<M> {
    fn cur_frame(&self) -> StackFrame<M> {
        self.active_thread().cur_frame()
    }

    fn mutate_cur_frame<O>(&mut self, f: impl FnOnce(&mut StackFrame<M>, &mut AtomicMemory<M>) -> NdResult<O>) -> NdResult<O> {
        self.threads.try_mutate_at(self.active_thread, |thread| thread.mutate_cur_frame(|frame| f(frame, &mut self.mem)))
    }

    fn mutate_cur_stack<O>(&mut self, f: impl FnOnce(&mut List<StackFrame<M>>) -> O) -> O {
        self.threads.mutate_at(self.active_thread, |thread| f(&mut thread.stack))
    }

    fn fn_from_addr(&self, addr: mem::Address) -> Result<Function> {
        let mut funcs = self.fn_addrs.iter().filter(|(_, fn_addr)| *fn_addr == addr);
        let Some((func_name, _)) = funcs.next() else {
            throw_ub!("Dereferencing function pointer where there is no function.");
        };
        let func = self.prog.functions[func_name];

        ret(func)
    }
}

impl<M: Memory> StackFrame<M> {
    /// jump to the beginning of the given block.
    fn jump_to_block(&mut self, b: BbName) {
        self.next_block = b;
        self.next_stmt = Int::ZERO;
    }
}
```

Next, we define how to create a thread.

```rust
impl<M: Memory> Machine<M> {
    fn new_thread(&mut self, func: Function, args: List<(Value<M>, PlaceType)>) -> NdResult<ThreadId> {
        // The bottom of a stack must have a 1-ZST return type.
        // This way it cannot assume there is actually a return place to write anything to.
        let init_frame = self.create_frame(
            func,
            ReturnAction::BottomOfStack,
            CallingConvention::C,
            unit_ptype(),
            args,
        )?;
        // Push the new thread, return the index.
        let thread = Thread {
            state: ThreadState::Enabled,
            stack: list![init_frame],
        };
        let thread_id = ThreadId::from(self.threads.len());
        self.threads.push(thread);
        ret(thread_id)
    }
}
```

Some functionality of the threads and the thread management in the machine.

```rust
impl<M: Memory> Thread<M> {
    fn cur_frame(&self) -> StackFrame<M> {
        self.stack.last().unwrap()
    }

    fn mutate_cur_frame<O>(&mut self, f: impl FnOnce(&mut StackFrame<M>) -> NdResult<O>) -> NdResult<O> {
        if self.stack.is_empty() {
            panic!("`mutate_cur_frame` called on empty stack!");
        }

        let last_idx = self.stack.len() - 1;
        self.stack.try_mutate_at(last_idx, f)
    }
}

impl<M: Memory> Machine<M> {
    fn active_thread(&self) -> Thread<M> {
        self.threads[self.active_thread]
    }
}
```
