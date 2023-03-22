#![cfg(test)]

extern crate gen_minirust;
extern crate miniutil;

pub use miniutil::run::*;
pub use miniutil::build::*;
pub use miniutil::fmt::*;

pub use gen_minirust::lang::*;
pub use gen_minirust::mem::*;
pub use gen_minirust::prelude::*;

pub use gen_minirust::libspecr::*;
pub use gen_minirust::libspecr::prelude::*;
pub use gen_minirust::libspecr::hidden::*;

pub use std::format;
pub use std::string::String;
pub use gen_minirust::prelude::NdResult;

mod pass;
mod ub;
mod ill_formed;

pub fn assert_stop(prog: Program) {
    assert_eq!(run_program(prog), TerminationInfo::MachineStop);
}

pub fn assert_ub(prog: Program, msg: &str) {
    assert_eq!(run_program(prog), TerminationInfo::Ub(gen_minirust::prelude::String::from_internal(msg.to_string())));
}

pub fn assert_ill_formed(prog: Program) {
    assert_eq!(run_program(prog), TerminationInfo::IllFormed);
}
