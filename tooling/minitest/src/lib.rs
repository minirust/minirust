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

pub fn assert_stop(prog: Program) {
    assert_eq!(run_program(prog), TerminationInfo::MachineStop);
}

pub fn assert_ub(prog: Program, msg: &str) {
    assert_eq!(run_program(prog), TerminationInfo::Ub(minirust_rs::prelude::String::from_internal(msg.to_string())));
}

pub fn assert_ill_formed(prog: Program) {
    assert_eq!(run_program(prog), TerminationInfo::IllFormed);
}
