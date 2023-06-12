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

mod rs {
    pub use rustc_hir::def_id::DefId;
    pub use rustc_middle::mir::UnevaluatedConst;
    pub use rustc_middle::mir::{interpret::*, *};
    pub use rustc_middle::ty::*;
    pub use rustc_mir_dataflow::storage::always_storage_live_locals;
    pub use rustc_target::abi::{call::*, Align, Size, FieldIdx};
}

extern crate gen_minirust;
extern crate miniutil;

pub use gen_minirust::libspecr::hidden::*;
pub use gen_minirust::libspecr::prelude::*;
pub use gen_minirust::libspecr::*;

pub use gen_minirust::lang::*;
pub use gen_minirust::mem::*;
pub use gen_minirust::prelude::NdResult;
pub use gen_minirust::prelude::*;

pub use std::format;
pub use std::string::String;

pub use miniutil::build;
pub use miniutil::fmt::dump_program;
pub use miniutil::run::*;

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

use std::collections::HashMap;
use std::path::Path;

fn main() {
    let file = std::env::args()
        .skip(1)
        .filter(|x| !x.starts_with('-'))
        .next()
        .unwrap_or_else(|| String::from("file.rs"));

    get_mini(file, |prog| {
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
