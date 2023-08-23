#![cfg(test)]

pub use miniutil::run::*;
pub use miniutil::build::*;
pub use miniutil::fmt::*;

pub use minirust_rs::libspecr::*;
pub use minirust_rs::libspecr::prelude::*;
pub use minirust_rs::libspecr::hidden::*;

pub use minirust_rs::lang::*;
pub use minirust_rs::mem::*;
pub use minirust_rs::prelude::*;
pub use minirust_rs::prelude::NdResult;

pub use std::format;
pub use std::string::String;

mod pass;
mod ub;
mod ill_formed;
mod deadlock;

#[track_caller]
pub fn assert_stop(prog: Program) {
    assert_eq!(run_program(prog), TerminationInfo::MachineStop);
}

#[track_caller]
pub fn assert_ub(prog: Program, msg: &str) {
    assert_eq!(run_program(prog), TerminationInfo::Ub(minirust_rs::prelude::String::from_internal(msg.to_string())));
}

#[track_caller]
pub fn assert_ill_formed(prog: Program) {
    assert_eq!(run_program(prog), TerminationInfo::IllFormed);
}

#[track_caller]
pub fn assert_deadlock(prog: Program) {
    assert_eq!(run_program(prog), TerminationInfo::Deadlock);
}


/// Run the program multiple times. Checks if we get a data race in some execution
/// This automatically fails if the program does not terminate correctly if the data race did not occur.
#[track_caller]
pub fn has_data_race(prog: Program) -> bool {
    let data_race_string = minirust_rs::prelude::String::from_internal("Data race".to_string());

    for _ in 0..20 {
        match run_program(prog) {
            TerminationInfo::MachineStop => {},
            TerminationInfo::Ub(ub) => {
                if ub == data_race_string {
                    return true;
                }
                panic!("Non data race undefined behavior");
            },
            termination_info => panic!("{:?}", termination_info)
        }
    }

    false
}
