use crate::build::*;

pub fn assign(destination: PlaceExpr, source: ValueExpr) -> Statement {
    Statement::Assign {
        destination,
        source,
    }
}

pub fn expose(value: ValueExpr) -> Statement {
    Statement::Expose { value }
}

pub fn validate(place: PlaceExpr, fn_entry: bool) -> Statement {
    Statement::Validate { place, fn_entry }
}

pub fn storage_live(x: u32) -> Statement {
    Statement::StorageLive(LocalName(Name::from_internal(x)))
}

pub fn storage_dead(x: u32) -> Statement {
    Statement::StorageDead(LocalName(Name::from_internal(x)))
}

pub fn goto(x: u32) -> Terminator {
    Terminator::Goto(BbName(Name::from_internal(x)))
}

pub fn if_(condition: ValueExpr, then_blk: u32, else_blk: u32) -> Terminator {
    Terminator::If {
        condition,
        then_block: BbName(Name::from_internal(then_blk)),
        else_block: BbName(Name::from_internal(else_blk)),
    }
}

pub fn unreachable() -> Terminator {
    Terminator::Unreachable
}

pub fn call(f: u32, args: &[ArgumentExpr], ret: Option<PlaceExpr>, next: Option<u32>) -> Terminator {
    Terminator::Call {
        callee: fn_ptr(f),
        arguments: args.iter().copied().collect(),
        ret: ret,
        next_block: next.map(|x| BbName(Name::from_internal(x))),
    }
}

pub fn by_value<T: TypeConv>(val: ValueExpr) -> ArgumentExpr {
    let pty = T::get_ptype();
    ArgumentExpr::ByValue(val, pty.align)
}

pub fn in_place(arg: PlaceExpr) -> ArgumentExpr {
    ArgumentExpr::InPlace(arg)
}

pub fn print(arg: ValueExpr, next: u32) -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::PrintStdout,
        arguments: list![arg],
        ret: None,
        next_block: Some(BbName(Name::from_internal(next))),
    }
}

pub fn eprint(arg: ValueExpr, next: u32) -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::PrintStderr,
        arguments: list![arg],
        ret: None,
        next_block: Some(BbName(Name::from_internal(next))),
    }
}

pub fn allocate(size: ValueExpr, align: ValueExpr, ret_place: PlaceExpr, next: u32) -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::Allocate,
        arguments: list![size, align],
        ret: Some(ret_place),
        next_block: Some(BbName(Name::from_internal(next))),
    }
}

pub fn deallocate(ptr: ValueExpr, size: ValueExpr, align: ValueExpr, next: u32) -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::Deallocate,
        arguments: list![ptr, size, align],
        ret: None,
        next_block: Some(BbName(Name::from_internal(next))),
    }
}

pub fn exit() -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::Exit,
        arguments: list![],
        ret: None,
        next_block: None,
    }
}

pub fn return_() -> Terminator {
    Terminator::Return
}

pub fn spawn(fn_ptr: ValueExpr, data_ptr: ValueExpr, ret: Option<PlaceExpr>, next: u32) -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::Spawn,
        arguments: list!(fn_ptr, data_ptr),
        ret,
        next_block: Some(BbName(Name::from_internal(next)))
    }
}

pub fn join(thread_id: ValueExpr, next: u32) -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::Join,
        arguments: list!(thread_id),
        ret: None,
        next_block: Some(BbName(Name::from_internal(next)))
    }
}

pub fn atomic_write(ptr: ValueExpr, src: ValueExpr, next: u32) -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::AtomicStore,
        arguments: list!(ptr, src),
        ret: None,
        next_block: Some(BbName(Name::from_internal(next)))
    }
}

pub fn atomic_read(dest: PlaceExpr, ptr: ValueExpr, next: u32) -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::AtomicLoad,
        arguments: list!(ptr),
        ret: Some(dest),
        next_block: Some(BbName(Name::from_internal(next)))
    }
}

pub fn compare_exchange(dest: PlaceExpr, ptr: ValueExpr, current: ValueExpr, next_val: ValueExpr, next: u32) -> Terminator {
    Terminator::CallIntrinsic { 
        intrinsic: Intrinsic::AtomicCompareExchange,
        arguments: list!(ptr, current, next_val),
        ret: Some(dest),
        next_block: Some(BbName(Name::from_internal(next)))
    }
}

pub fn create_lock(ret: PlaceExpr, next: u32) -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::Lock(LockIntrinsic::Create),
        arguments: list!(),
        ret: Some(ret),
        next_block: Some(BbName(Name::from_internal(next)))
    }
}

pub fn acquire(lock_id: ValueExpr, next: u32) -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::Lock(LockIntrinsic::Acquire),
        arguments: list!(lock_id),
        ret: None,
        next_block: Some(BbName(Name::from_internal(next)))
    }
}

pub fn release(lock_id: ValueExpr, next: u32) -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::Lock(LockIntrinsic::Release),
        arguments: list!(lock_id),
        ret: None,
        next_block: Some(BbName(Name::from_internal(next)))
    }
}
