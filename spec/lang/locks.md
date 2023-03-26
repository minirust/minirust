# Machine Locks

This file describes how system locks work in MiniRust.
This file might be temporary for testing purposes, since rust currently implements locks via futex and not system locks.

## The Lock State

```rust
pub enum LockState {
    Unlocked,
    LockedBy(ThreadId),
}

type LockId = Int;
```

We implement locks for the thread manager. Since they are most useful for exactly that purpose.

```rust
impl<M: Memory> ThreadManager<M> {
    pub fn lock_create(&mut self) -> LockId {
        let id = self.locks.len();

        self.locks.push(LockState::Unlocked);

        id
    }

    pub fn lock_acquire(&mut self, lock_id: LockId) -> Result {
        let active = self.active_thread.unwrap();

        let Some(lock) = self.locks.get(lock_id) else {
            throw_ub!("Acquiring non existing lock");
        };

        match lock {
            LockState::Unlocked => {
                self.locks.mutate_at(lock_id, |lock_state| {
                    *lock_state = LockState::LockedBy(active);
                });
            },
            LockState::LockedBy(_) => {
                self.threads.mutate_at(active, |thread| {
                    thread.state = ThreadState::BlockedOnLock(lock_id);
                });
            },
        }

        ret(())
    }

    pub fn lock_release(&mut self, lock_id: LockId) -> NdResult {
        let active = self.active_thread.unwrap();

        let Some(lock) = self.locks.get(lock_id) else {
            throw_ub!("Release non existing lock");
        };

        match lock {
            LockState::LockedBy(thread_id) if thread_id == active => {
                if self.threads.any(|thread| thread.state == ThreadState::BlockedOnLock(lock_id)) {
                    let distr = libspecr::IntDistribution {
                        start: Int::ZERO,
                        end: Int::from(self.threads.len()),
                        divisor: Int::ONE,
                    };

                    let thread_id: ThreadId = pick(distr, |id: ThreadId| {
                        let Some(thread) = self.threads.get(id) else {
                            return false;
                        };

                        thread.state == ThreadState::BlockedOnLock(lock_id)
                    })?;

                    self.threads.mutate_at(thread_id, |thread| {
                        thread.state = ThreadState::Enabled;
                    });

                    self.locks.mutate_at(lock_id, |lock| {
                        *lock = LockState::LockedBy(thread_id);
                    });
                }
                
                else {
                    self.locks.mutate_at(lock_id, |lock| {
                        *lock = LockState::Unlocked;
                    });
                }


                ret(())
            },
            _ => throw_ub!("Releasing non owned lock.")
        }
    }
}
```

## The Intrinsics for Locks

Since the locks might be temporary they are mostly restricted to this file. Therefore it is better to define the intrinsics here.

```rust
impl<M: Memory> Machine<M> {
    #[specr::argmatch(lock_intrinsic)]
    fn eval_lock_intrinsic(
        &mut self,
        lock_intrinsic: LockIntrinsic,
        arguments: List<Value<M>>,
    ) -> NdResult<(Value<M>, Type)> { .. }
}
```

The `Create` intrinsic. Used to create locks.

```rust
impl<M: Memory> Machine<M> {
    fn eval_lock_intrinsic(
        &mut self,
        LockIntrinsic::Create: LockIntrinsic,
        arguments: List<Value<M>>,
    ) -> NdResult<(Value<M>, Type)> {
        if arguments.len() > 0 {
            throw_ub!("Invalid number of arguments for `LockIntrinsic::Create`");
        }

        let lock_id = self.thread_manager.lock_create();

        // FIXME: What if the id does not fit into a u32.
        // Currently the id is just a u32. This is hardcoded but no real thought is behind this.
        let id_size = Size::from_bits(Int::from(32)).unwrap();
        let id_type = Type::Int(IntType{
            signed: Signedness::Unsigned,
            size: id_size,
        });

        ret((Value::Int(lock_id), id_type))
    }
}
```

The `Acquire` intrinsic.

```rust
impl<M: Memory> Machine<M> {
    fn eval_lock_intrinsic(
        &mut self,
        LockIntrinsic::Acquire: LockIntrinsic,
        arguments: List<Value<M>>,
    ) -> NdResult<(Value<M>, Type)> {
        if arguments.len() != 1 {
            throw_ub!("Invalid number of arguments for `LockIntrinsic::Acquire`");
        }

        let Value::Int(lock_id) = arguments[0] else {
            throw_ub!("Invalid first argument to `LockIntrinsic::Acquire`");
        };

        self.thread_manager.lock_acquire(lock_id)?;

        ret(unit())
    }
}
```

The `Release` intrinsic.

```rust
impl<M: Memory> Machine<M> {
    fn eval_lock_intrinsic(
        &mut self,
        LockIntrinsic::Release: LockIntrinsic,
        arguments: List<Value<M>>,
    ) -> NdResult<(Value<M>, Type)> {
        if arguments.len() != 1 {
            throw_ub!("Invalid number of arguments for `LockIntrinsic::Release`");
        }

        let Value::Int(lock_id) = arguments[0] else {
            throw_ub!("Invalid first argument to `LockIntrinsic::Release`");
        };

        self.thread_manager.lock_release(lock_id)?;

        ret(unit())
    }
}
```
