# Machine Locks

This file describes how system locks work in MiniRust.
This does not match the actual lock implementations in Rust, it serves more as a specification for idealized locks.

## The Lock State

```rust
pub enum LockState {
    Unlocked,
    LockedBy(ThreadId),
}

type LockId = Int;
```

## Lock operations

The Machine provides the key operations on locks.

```rust
impl<M: Memory> Machine<M> {
    pub fn lock_create(&mut self) -> LockId {
        let id = self.locks.len();

        self.locks.push(LockState::Unlocked);

        id
    }

    pub fn lock_acquire(&mut self, lock_id: LockId) -> Result {
        let active = self.active_thread;

        let Some(lock) = self.locks.get(lock_id) else {
            throw_ub!("acquiring non-existing lock");
        };

        // If the lock is not taken, the lock gets acquired by the current (active) thread.
        // Otherwise, the active thread gets blocked.
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
        let active = self.active_thread;

        let Some(lock) = self.locks.get(lock_id) else {
            throw_ub!("releasing non-existing lock");
        };

        match lock {
            LockState::LockedBy(thread_id) if thread_id == active => {
                // If any thread is blocked waiting for this lock, we want to unblock one of those.
                if self.threads.any(|thread| thread.state == ThreadState::BlockedOnLock(lock_id)) {
                    // We pick the thread that gets the lock.
                    let distr = libspecr::IntDistribution {
                        start: Int::ZERO,
                        end: Int::from(self.threads.len()),
                        divisor: Int::ONE,
                    };

                    let acquirer_id: ThreadId = pick(distr, |id: ThreadId| {
                        let Some(thread) = self.threads.get(id) else {
                            return false;
                        };

                        thread.state == ThreadState::BlockedOnLock(lock_id)
                    })?;

                    // We unblock the selected thread.
                    self.threads.mutate_at(acquirer_id, |thread| {
                        thread.state = ThreadState::Enabled;
                    });

                    // The acquirer got synchronized because it got enabled by this thread.
                    self.synchronized_threads.insert(acquirer_id);

                    // Rather than unlock and lock again we just change the lock owner.
                    self.locks.mutate_at(lock_id, |lock| {
                        *lock = LockState::LockedBy(acquirer_id);
                    });
                }

                else {
                    self.locks.mutate_at(lock_id, |lock| {
                        *lock = LockState::Unlocked;
                    });
                }


                ret(())
            },
            _ => throw_ub!("releasing non-acquired lock")
        }
    }
}
```

## The Intrinsics for Locks

This exposes the Machine operations for locks to the language as intrinsics.

The `Create` intrinsic. Used to create locks.

```rust
impl<M: Memory> Machine<M> {
    fn eval_intrinsic(
        &mut self,
        Intrinsic::Lock(LockIntrinsic::Create): Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if arguments.len() > 0 {
            throw_ub!("invalid number of arguments for `LockIntrinsic::Create`");
        }

        if !matches!(ret_ty, Type::Int(_)) {
            throw_ub!("invalid return type for `LockIntrinsic::Create`")
        }

        let lock_id = self.lock_create();

        ret(Value::Int(lock_id))
    }
}
```

The `Acquire` intrinsic.

```rust
impl<M: Memory> Machine<M> {
    fn eval_intrinsic(
        &mut self,
        Intrinsic::Lock(LockIntrinsic::Acquire): Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if arguments.len() != 1 {
            throw_ub!("invalid number of arguments for `LockIntrinsic::Acquire`");
        }

        let Value::Int(lock_id) = arguments[0].0 else {
            throw_ub!("invalid first argument to `LockIntrinsic::Acquire`");
        };

        if ret_ty != unit_type() {
            throw_ub!("invalid return type for `LockIntrinsic::Acquire`")
        }

        self.lock_acquire(lock_id)?;

        ret(unit_value())
    }
}
```

The `Release` intrinsic.

```rust
impl<M: Memory> Machine<M> {
    fn eval_intrinsic(
        &mut self,
        Intrinsic::Lock(LockIntrinsic::Release): Intrinsic,
        arguments: List<(Value<M>, Type)>,
        ret_ty: Type,
    ) -> NdResult<Value<M>> {
        if arguments.len() != 1 {
            throw_ub!("invalid number of arguments for `LockIntrinsic::Release`");
        }

        let Value::Int(lock_id) = arguments[0].0 else {
            throw_ub!("invalid first argument to `LockIntrinsic::Release`");
        };

        if ret_ty != unit_type() {
            throw_ub!("invalid return type for `LockIntrinsic::Release`")
        }

        self.lock_release(lock_id)?;

        ret(unit_value())
    }
}
```
