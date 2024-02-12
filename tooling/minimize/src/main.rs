#![feature(rustc_private)]
#![feature(box_patterns)]
#![feature(never_type)]
// This is required since `get::Cb` contained `Option<Program>`.
#![recursion_limit = "256"]

extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_mir_dataflow;
extern crate rustc_target;
extern crate rustc_session;

mod rs {
    pub use rustc_hir::def_id::DefId;
    pub use rustc_middle::mir::UnevaluatedConst;
    pub use rustc_middle::mir::{interpret::*, *};
    pub use rustc_middle::ty::*;
    pub use rustc_mir_dataflow::storage::always_storage_live_locals;
    pub use rustc_target::abi::{call::*, Align, Size, FieldIdx};
}

pub use minirust_rs::libspecr::hidden::*;
pub use minirust_rs::libspecr::prelude::*;
pub use minirust_rs::libspecr::*;

pub use minirust_rs::lang::*;
pub use minirust_rs::mem::*;
pub use minirust_rs::prelude::*;
pub use minirust_rs::prelude::NdResult;

pub use std::format;
pub use std::string::String;

pub use miniutil::build::{self, TypeConv as _};
pub use miniutil::fmt::dump_program;
pub use miniutil::run::*;
pub use miniutil::DefaultTarget;

mod program;
use program::*;

mod ty;
use ty::*;

mod bb;
use bb::*;

mod rvalue;
use rvalue::*;

mod constant;
use constant::*;

mod get;
use get::get_mini;

mod chunks;
use chunks::calc_chunks;

mod enums;
use enums::{translate_enum, discriminant_for_variant, int_from_bits};

use std::collections::HashMap;

fn main() {
    get_mini(|prog| {
        let dump = std::env::args().skip(1).any(|x| x == "--dump");
        if dump {
            dump_program(prog);
        } else {
            match run_program(prog) {
                TerminationInfo::IllFormed => eprintln!("ERR: program not well-formed."),
                TerminationInfo::MachineStop => { /* silent exit. */ }
                TerminationInfo::Ub(err) => eprintln!("UB: {}", err.get_internal()),
                _ => unreachable!(),
            }
        }
    });
}
