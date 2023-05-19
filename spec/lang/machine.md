# MiniRust Abstract Machine

This defines the state that makes up the MiniRust Abstract Machine:
which components together make up the state of a MiniRust program during its execution?
This key data structure says a lot about how the Abstract Machine is structured.
(The "reduction relation" aka operational semantics aka `step` function is defined in [the next file](step.md).)

```rust
/// This type contains everything that needs to be tracked during the execution
/// of a MiniRust program.
#[no_auto_derive]
#[derive(GcCompat, Clone)]
pub struct Machine<M: Memory> {
    /// The program we are executing.
    prog: Program,

    /// The state of memory.
    mem: M,

    /// The state of the integer-pointer cast subsystem.
    intptrcast: IntPtrCast<M::Provenance>,

    /// The Thread Manager
    thread_manager: ThreadManager<M>,

    /// Stores a pointer to each of the global allocations.
    global_ptrs: Map<GlobalName, Pointer<M::Provenance>>,

    /// Stores an address for each function name.
    fn_addrs: Map<FnName, mem::Address>,

    out: DynWrite,
    err: DynWrite,
}

/// The data that makes up a stack frame.
struct StackFrame<M: Memory> {
    /// The function this stack frame belongs to.
    func: Function,

    /// For each live local, the place in memory where its value is stored.
    locals: Map<LocalName, Place<M>>,

    /// Expresses what the caller does after the callee (this function) returns.
    /// If `None` this is the bottommost stack frame.
    caller_return_info: Option<CallerReturnInfo<M>>,

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

/// The ID of a thread is an index into the ThreadManager's `threads` list.
type ThreadId = Int;

/// The thread manager tracks the list of all threads, and the thread that is currently taking a step.
/// The latter is only needed during a step of execution; 
/// it saves us from passing the active thread around explicitly everywhere.
pub struct ThreadManager<M: Memory> {
    /// The list of threads.
    threads: List<Thread<M>>,

    /// The list of locks.
    locks: List<LockState>,

    /// To avoid passing around the active thread through all the eval_ functions,
    /// we store it globally here.
    active_thread: Option<ThreadId>,
}
```

Next, we define how to create a machine.

```rust
impl<M: Memory> Machine<M> {
    pub fn new(prog: Program, out: DynWrite, err: DynWrite) -> NdResult<Machine<M>> {
        if prog.check_wf::<M>().is_none() {
            throw_ill_formed!();
        }

        let mut mem = M::new();
        let mut global_ptrs = Map::new();
        let mut fn_addrs = Map::new();

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

        // Allocate functions.
        for (fn_name, _function) in prog.functions {
            let alloc = mem.allocate(Size::ZERO, Align::ONE)?;
            let addr = alloc.addr;
            // Ensure that no two functions lie on the same address.
            assert!(!fn_addrs.values().any(|fn_addr| addr == fn_addr));
            fn_addrs.insert(fn_name, addr);
        }

        let start_fn = prog.functions[prog.start];

        ret(Machine {
            prog,
            mem,
            intptrcast: IntPtrCast::new(),
            global_ptrs,
            fn_addrs,
            thread_manager: ThreadManager::new(start_fn),
            out,
            err,
        })
    }
}
```

We also define some helper functions that will be useful later.

```rust
impl<M: Memory> Machine<M> {
    fn cur_frame(&self) -> StackFrame<M> {
        let Some(active_thread) = self.thread_manager.active_thread else {
            panic!("`cur_frame` called without active thread!");
        };

        self.thread_manager.threads[active_thread].cur_frame()
    }

    fn mutate_cur_frame<O>(&mut self, f: impl FnOnce(&mut StackFrame<M>) -> O) -> O {
        let Some(active_thread) = self.thread_manager.active_thread else {
            panic!("`mutate_cur_frame` called without active thread!");
        };

        self.thread_manager.threads.mutate_at(active_thread, |thread| thread.mutate_cur_frame(f))
    }

    fn mutate_cur_stack<O>(&mut self, f: impl FnOnce(&mut List<StackFrame<M>>) -> O) -> O {
        let Some(active_thread) = self.thread_manager.active_thread else {
            panic!("`cur_stack` called without active thread!");
        };

        self.thread_manager.threads.mutate_at(active_thread, |thread| f(&mut thread.stack))
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
impl<M: Memory> Thread<M> {
    fn new(func: Function) -> Self {
        // Setup the initial stack frame.
        // For the main thread, well-formedness ensures that the func has
        // no return value and no arguments.
        // For any other threads, the spawn intrinsic ensures
        // that the func has no arguments.
        let init_frame = StackFrame {
            func,
            locals: Map::new(),
            caller_return_info: None,
            next_block: func.start,
            next_stmt: Int::ZERO,
        };

        Self {
            state: ThreadState::Enabled,
            stack: list![init_frame],
        }
    }
}
```

Some functionality of the threads and the thread manager.

```rust
impl<M: Memory> Thread<M> {
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

// Some helper functions.
impl<M: Memory> ThreadManager<M> {
    fn cur_thread(&self) -> Thread<M> {
        let Some(active_thread) = self.active_thread else {
            panic!("`cur_thread` called without active thread!");
        };

        self.threads[active_thread]
    }

    fn mutate_cur_thread<O>(&mut self, f: impl FnOnce(&mut Thread<M>) -> O) -> O {
        let Some(active_thread) = self.active_thread else {
            panic!("`mut_cur_thread` called without active thread!");
        };

        self.threads.mutate_at(active_thread, f)
    } 
}

impl<M: Memory> ThreadManager<M> {
    pub fn new(func: Function) -> Self {
        let master = Thread::new(func);

        let mut threads = List::new();
        threads.push(master);

        Self {
            threads,
            locks: List::new(),
            active_thread: None,
        }
    }

    pub fn spawn(&mut self, func: Function) -> NdResult<ThreadId> {
        let thread_id = ThreadId::from(self.threads.len());
        self.threads.push(Thread::new(func));
        ret(thread_id)
    }

    pub fn join(&mut self, thread_id: ThreadId) -> NdResult {
        let Some(thread) = self.threads.get(thread_id) else {
            throw_ub!("`Intrinsic::Join`: join non existing thread");
        };

        match thread.state {
            ThreadState::Terminated => {},
            _ => {
                self.mutate_cur_thread(|thread|{
                    thread.state = ThreadState::BlockedOnJoin(thread_id);
                });
            },
        };

        ret(())
    }

    pub fn terminate_active_thread(&mut self) -> NdResult {
        let active = self.active_thread.unwrap();

        if active == 0 {
            // The main thread terminating stops the machine.
            throw_machine_stop!();
        }

        self.threads.mutate_at(active, |thread| thread.state = ThreadState::Terminated);

        self.threads = self.threads.into_iter().map(|mut thread| {
            match thread.state {
                ThreadState::BlockedOnJoin(join_id) if join_id == active => {
                    thread.state = ThreadState::Enabled
                },
                _ => {}
            }
            thread
        }).collect();

        ret(())
    }
}
```
