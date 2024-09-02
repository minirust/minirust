#![cfg(test)]

pub use miniutil::build::*;
pub use miniutil::fmt::*;
pub use miniutil::run::*;
pub use miniutil::BasicMem;

pub use minirust_rs::libspecr::hidden::*;
pub use minirust_rs::libspecr::prelude::*;
pub use minirust_rs::libspecr::*;

pub use minirust_rs::lang::*;
pub use minirust_rs::mem::*;
pub use minirust_rs::prelude::NdResult;
pub use minirust_rs::prelude::*;

pub use std::format;
pub use std::string::String;

mod tests;

#[track_caller]
pub fn assert_stop<M: Memory>(prog: Program) {
    assert_eq!(run_program::<M>(prog), TerminationInfo::MachineStop);
}

#[track_caller]
pub fn assert_stop_always<M: Memory>(prog: Program, attempts: usize) {
    for _ in 0..attempts {
        assert_eq!(run_program::<M>(prog), TerminationInfo::MachineStop);
    }
}

#[track_caller]
pub fn assert_abort<M: Memory>(prog: Program, msg: &str) {
    let msg = prelude::String::from_internal(msg.to_string());
    assert_eq!(run_program::<M>(prog), TerminationInfo::Abort(msg));
}

#[track_caller]
pub fn assert_ub<M: Memory>(prog: Program, msg: &str) {
    assert_eq!(
        run_program::<M>(prog),
        TerminationInfo::Ub(minirust_rs::prelude::String::from_internal(msg.to_string()))
    );
}

#[track_caller]
pub fn assert_ub_eventually<M: Memory>(prog: Program, attempts: usize, msg: &str) {
    let msg = minirust_rs::prelude::String::from_internal(msg.to_string());
    for _ in 0..attempts {
        match run_program::<M>(prog) {
            TerminationInfo::MachineStop => continue,
            TerminationInfo::Ub(res) if res == msg => {
                // Got the expected result.
                return;
            }
            termination_info => {
                panic!("unexpected outcome in `assert_ub_eventually`: {:?}", termination_info);
            }
        }
    }
    panic!("did not get expected output after {} attempts", attempts);
}

/// Create program that assigns `expr` to local of type T and checks if it causes UB.
#[track_caller]
pub fn assert_ub_expr<T: TypeConv, M: Memory>(expr: ValueExpr, msg: &str) {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let local = f.declare_local::<T>();

    f.storage_live(local);
    f.assign(local, expr);
    f.exit();

    let f = p.finish_function(f);
    let p = p.finish_program(f);
    assert_ub::<M>(p, msg);
}

#[track_caller]
pub fn assert_ill_formed<M: Memory>(prog: Program, msg: &str) {
    let TerminationInfo::IllFormed(info) = run_program::<M>(prog) else {
        panic!("program is not ill formed!")
    };
    assert_eq!(info.get_internal(), msg, "program is ill-formed with a different error message");
}

#[track_caller]
pub fn assert_deadlock<M: Memory>(prog: Program) {
    assert_eq!(run_program::<M>(prog), TerminationInfo::Deadlock);
}

#[track_caller]
pub fn assert_memory_leak<M: Memory>(prog: Program) {
    assert_eq!(run_program::<M>(prog), TerminationInfo::MemoryLeak);
}

/// Run the program multiple times. Checks if we get a data race in some execution
/// This automatically fails if the program does not terminate correctly if the data race did not occur.
#[track_caller]
pub fn has_data_race<M: Memory>(prog: Program) -> bool {
    let data_race_string = minirust_rs::prelude::String::from_internal("data race".to_string());

    for _ in 0..32 {
        match run_program::<M>(prog) {
            TerminationInfo::MachineStop => {}
            TerminationInfo::Ub(ub) if ub == data_race_string => {
                return true;
            }
            termination_info => {
                panic!("unexpected outcome in `has_data_race`: {:?}", termination_info);
            }
        }
    }

    false
}
