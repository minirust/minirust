use crate::build::*;

pub fn assign(destination: PlaceExpr, source: ValueExpr) -> Statement {
    Statement::Assign { destination, source }
}

pub fn finalize(place: PlaceExpr, fn_entry: bool) -> Statement {
    Statement::Finalize { place, fn_entry }
}

pub fn live(x: u32) -> Statement {
    Statement::StorageLive(LocalName(Name::new(x)))
}

pub fn dead(x: u32) -> Statement {
    Statement::StorageDead(LocalName(Name::new(x)))
}

pub fn goto(x: u32) -> Terminator {
    Terminator::Goto(BbName(Name::new(x)))
}

pub fn if_(condition: ValueExpr, then_blk: u32, else_blk: u32) -> Terminator {
    Terminator::If {
        condition,
        then_block: BbName(Name::new(then_blk)),
        else_block: BbName(Name::new(else_blk)),
    }
}

pub fn unreachable() -> Terminator {
    Terminator::Unreachable
}

pub fn call(f: u32, args: &[ValueExpr], ret: Option<PlaceExpr>, next: Option<u32>) -> Terminator {
    Terminator::Call {
        callee: fn_ptr(f),
        arguments: args.iter().map(|x| (*x, ArgAbi::Register)).collect(),
        ret: ret.map(|x| (x, ArgAbi::Register)),
        next_block: next.map(|x| BbName(Name::new(x))),
    }
}

pub fn print(arg: ValueExpr, next: u32) -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::PrintStdout,
        arguments: list![arg],
        ret: None,
        next_block: Some(BbName(Name::new(next))),
    }
}

pub fn eprint(arg: ValueExpr, next: u32) -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::PrintStderr,
        arguments: list![arg],
        ret: None,
        next_block: Some(BbName(Name::new(next))),
    }
}

pub fn allocate(size: ValueExpr, align: ValueExpr, ret_place: PlaceExpr, next: u32) -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::Allocate,
        arguments: list![size, align],
        ret: Some(ret_place),
        next_block: Some(BbName(Name::new(next))),
    }
}

pub fn deallocate(ptr: ValueExpr, size: ValueExpr, align: ValueExpr, next: u32) -> Terminator {
    Terminator::CallIntrinsic {
        intrinsic: Intrinsic::Deallocate,
        arguments: list![ptr, size, align],
        ret: None,
        next_block: Some(BbName(Name::new(next))),
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
