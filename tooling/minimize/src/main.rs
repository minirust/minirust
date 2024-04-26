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
extern crate rustc_session;
extern crate rustc_smir;
extern crate rustc_span;
extern crate rustc_target;
extern crate stable_mir;

mod rs {
    pub use rustc_middle::mir::UnevaluatedConst;
    pub use rustc_middle::mir::{self, interpret::*, *};
    pub use rustc_middle::span_bug;
    pub use rustc_middle::ty::*;
    pub use rustc_mir_dataflow::storage::always_storage_live_locals;
    pub use rustc_span::source_map::Spanned;
    pub use rustc_span::Span;
    pub use rustc_target::abi::{call::*, Align, FieldIdx, Layout, Size};
}

mod smir {
    pub use rustc_smir::rustc_internal::*;
    pub use stable_mir::mir::mono::*;
    pub use stable_mir::mir::*;
    pub use stable_mir::ty::*;
}

pub use minirust_rs::libspecr::hidden::*;
pub use minirust_rs::libspecr::prelude::*;
pub use minirust_rs::libspecr::*;

pub use minirust_rs::lang::*;
pub use minirust_rs::mem::*;
pub use minirust_rs::prelude::NdResult;
pub use minirust_rs::prelude::*;

use std::env::Args;
pub use std::format;
pub use std::string::String;

pub use miniutil::build::{self, TypeConv as _};
pub use miniutil::fmt::dump_program;
pub use miniutil::run::*;
pub use miniutil::DefaultTarget;

mod program;
use program::*;

mod function;
use function::*;

mod ty;
use ty::*;

mod bb;

mod rvalue;

mod constant;

mod get;
use get::get_mini;

mod chunks;
use chunks::calc_chunks;

mod enums;
use enums::int_from_bits;

use std::collections::HashMap;

fn main() {
    let (minimize_args, rustc_args) = split_args(std::env::args());
    let dump = minimize_args.iter().any(|x| x == "--minimize-dump");
    get_mini(rustc_args, |tcx, prog| {
        if dump {
            dump_program(prog);
        } else {
            match run_program(prog) {
                TerminationInfo::IllFormed =>
                    tcx.dcx().fatal("ERR: program not well-formed (this is a bug in minimize)"),
                TerminationInfo::MachineStop => { /* silent exit. */ }
                TerminationInfo::Ub(err) => tcx.dcx().fatal(format!("UB: {}", err.get_internal())),
                TerminationInfo::Deadlock => tcx.dcx().fatal("program dead-locked"),
                TerminationInfo::MemoryLeak => tcx.dcx().fatal("program leaked memory"),
            }
        }
    });
}

/// split arguments into arguments for minimize and rustc
fn split_args(args: Args) -> (Vec<String>, Vec<String>) {
    let mut minimize_args: Vec<String> = Vec::new();
    let mut rustc_args: Vec<String> = Vec::new();
    for arg in args {
        if arg.starts_with("--minimize-") {
            minimize_args.push(arg);
        } else {
            rustc_args.push(arg);
        }
    }
    (minimize_args, rustc_args)
}
