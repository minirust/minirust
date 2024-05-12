#![cfg(test)]

pub use miniutil::build::*;
pub use miniutil::fmt::*;
pub use miniutil::run::*;

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
pub fn assert_stop(prog: Program) {
    assert_eq!(run_program(prog), TerminationInfo::MachineStop);
}

#[track_caller]
pub fn assert_stop_always(prog: Program, attempts: usize) {
    for _ in 0..attempts {
        assert_eq!(run_program(prog), TerminationInfo::MachineStop);
    }
}

#[track_caller]
pub fn assert_ub(prog: Program, msg: &str) {
    assert_eq!(
        run_program(prog),
        TerminationInfo::Ub(minirust_rs::prelude::String::from_internal(msg.to_string()))
    );
}

#[track_caller]
pub fn assert_ub_eventually(prog: Program, attempts: usize, msg: &str) {
    let msg = minirust_rs::prelude::String::from_internal(msg.to_string());
    for _ in 0..attempts {
        match run_program(prog) {
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

#[track_caller]
pub fn assert_ill_formed(prog: Program, msg: &str) {
    let TerminationInfo::IllFormed(info) = run_program(prog) else {
        panic!("program is not ill formed!")
    };
    assert_eq!(info.get_internal(), msg, "program is ill-formed with a different error message");
}

#[track_caller]
pub fn assert_deadlock(prog: Program) {
    assert_eq!(run_program(prog), TerminationInfo::Deadlock);
}

#[track_caller]
pub fn assert_memory_leak(prog: Program) {
    assert_eq!(run_program(prog), TerminationInfo::MemoryLeak);
}

/// Run the program multiple times. Checks if we get a data race in some execution
/// This automatically fails if the program does not terminate correctly if the data race did not occur.
#[track_caller]
pub fn has_data_race(prog: Program) -> bool {
    let data_race_string = minirust_rs::prelude::String::from_internal("Data race".to_string());

    for _ in 0..32 {
        match run_program(prog) {
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
