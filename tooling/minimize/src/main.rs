#![feature(rustc_private)]
#![feature(box_patterns)]
#![feature(never_type)]
// This is required since `get::Cb` contained `Option<Program>`.
#![recursion_limit = "256"]

// Imports for the rest of the crate

extern crate rustc_const_eval;
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
    pub use rustc_const_eval::const_eval::mk_eval_cx_for_const_val;
    pub use rustc_const_eval::interpret::{InterpCx, OpTy};
    pub use rustc_middle::mir::{self, interpret::*, *};
    pub use rustc_middle::span_bug;
    pub use rustc_middle::ty::*;
    pub use rustc_mir_dataflow::storage::always_storage_live_locals;
    pub use rustc_span::source_map::Spanned;
    pub use rustc_span::{DUMMY_SP, Span, sym};
    pub use rustc_target::abi::{self, Align, FieldIdx, Layout, Size, call::*};
    pub use rustc_target::abi::{FieldsShape, TagEncoding, VariantIdx, Variants};

    pub type CompileTimeInterpCx<'tcx> =
        InterpCx<'tcx, rustc_const_eval::const_eval::CompileTimeMachine<'tcx>>;
}
// Traits
pub use rustc_middle::ty::layout::IntegerExt as _;
pub use rustc_target::abi::HasDataLayout as _;

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

pub use miniutil::BasicMem;
pub use miniutil::DefaultTarget;
pub use miniutil::TreeBorrowMem;
pub use miniutil::build::{self, TypeConv as _, unit_place};
pub use miniutil::fmt::dump_program;
pub use miniutil::run::*;

// Get back some `std` items
pub use std::format;
pub use std::string::String;

mod program;
use program::*;

mod function;
use function::*;

mod ty;
use ty::*;

mod bb;

mod rvalue;

mod constant;

mod chunks;
use chunks::calc_chunks;

mod enums;
use enums::int_from_bits;

// Imports for `main``

use std::collections::HashMap;
use std::env::Args;

pub const DEFAULT_ARGS: &[&str] = &[
    // This is the same as Miri's `MIRI_DEFAULT_ARGS`, ensuring we get a MIR with all the UB still present.
    "--cfg=miri",
    "-Zalways-encode-mir",
    "-Zextra-const-ub-checks",
    "-Zmir-emit-retag",
    "-Zmir-opt-level=0",
    "-Zmir-enable-passes=-CheckAlignment",
    "-Zmir-keep-place-mention",
    // Also disable UB checks (since `cfg(miri)` in the standard library do not trigger for us).
    "-Zub-checks=false",
];

fn show_error(msg: &impl std::fmt::Display) -> ! {
    eprintln!("fatal error: {msg}");
    std::process::exit(101) // exit code needed to make ui_test happy
}

macro_rules! show_error {
    ($($tt:tt)*) => { crate::show_error(&format_args!($($tt)*)) };
}

fn main() {
    let (minimize_args, rustc_args) = split_args(std::env::args());
    let dump = minimize_args.iter().any(|x| x == "--minimize-dump");

    get_mini(rustc_args, |_tcx, prog| {
        if dump {
            dump_program(prog);
        } else {
            match run_prog(prog, &minimize_args) {
                // We can't use tcx.dcx().fatal due to <https://github.com/oli-obk/ui_test/issues/226>
                TerminationInfo::IllFormed(err) =>
                    show_error!(
                        "program not well-formed (this is a bug in minimize):\n    {}",
                        err.get_internal()
                    ),
                TerminationInfo::MachineStop => { /* silent exit. */ }
                TerminationInfo::Abort(err) => show_error!("Panic: {}", err.get_internal()),
                TerminationInfo::Ub(err) => show_error!("UB: {}", err.get_internal()),
                TerminationInfo::Deadlock => show_error!("program dead-locked"),
                TerminationInfo::MemoryLeak => show_error!("program leaked memory"),
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

fn run_prog(prog: Program, args: &Vec<String>) -> TerminationInfo {
    if args.iter().any(|x| x == "--minimize-tree-borrows") {
        run_program::<TreeBorrowMem>(prog)
    } else {
        run_program::<BasicMem>(prog)
    }
}

fn get_mini(mut args: Vec<String>, callback: impl FnOnce(rs::TyCtxt<'_>, Program) + Send + Copy) {
    args.splice(1..1, DEFAULT_ARGS.iter().map(ToString::to_string));
    rustc_driver::RunCompiler::new(&args, &mut Cb { callback }).run().unwrap();
}

struct Cb<F: FnOnce(rs::TyCtxt<'_>, Program) + Send + Copy> {
    callback: F,
}

impl<F: FnOnce(rs::TyCtxt<'_>, Program) + Send + Copy> rustc_driver::Callbacks for Cb<F> {
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        queries: &'tcx rustc_interface::Queries<'tcx>,
    ) -> rustc_driver::Compilation {
        queries.global_ctxt().unwrap().enter(|tcx| {
            // StableMIR can only be used inside a `run` call, to guarantee its context is properly
            // initialized. Calls to StableMIR functions will panic if done outside a run.
            let prog = smir::run(tcx, || Ctxt::new(tcx).translate()).unwrap();
            (self.callback)(tcx, prog);
        });

        rustc_driver::Compilation::Stop
    }
}
