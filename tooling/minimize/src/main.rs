#![feature(rustc_private)]
#![feature(box_patterns)]
#![feature(never_type)]
#![feature(strict_overflow_ops)]
#![feature(array_windows)]
// This is required since `get::Cb` contained `Option<Program>`.
#![recursion_limit = "256"]

// Imports for the rest of the crate

extern crate rustc_abi;
extern crate rustc_const_eval;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_index;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_mir_dataflow;
extern crate rustc_session;
extern crate rustc_smir;
extern crate rustc_span;
extern crate rustc_target;
extern crate stable_mir;

mod rs {
    pub use rustc_abi as abi;
    pub use rustc_abi::{
        Align, FieldIdx, FieldsShape, Layout, Size, TagEncoding, VariantIdx, Variants,
    };
    pub use rustc_const_eval::const_eval::mk_eval_cx_for_const_val;
    pub use rustc_const_eval::interpret::{InterpCx, OpTy};
    pub use rustc_middle::mir::{self, interpret::*, *};
    pub use rustc_middle::span_bug;
    pub use rustc_middle::ty::*;
    pub use rustc_mir_dataflow::impls::always_storage_live_locals;
    pub use rustc_span::source_map::Spanned;
    pub use rustc_span::{DUMMY_SP, Span, sym};
    pub use rustc_target::callconv::{Conv, FnAbi};

    pub type CompileTimeInterpCx<'tcx> =
        InterpCx<'tcx, rustc_const_eval::const_eval::CompileTimeMachine<'tcx>>;
}

// Traits
pub use rustc_abi::HasDataLayout as _;
pub use rustc_middle::ty::layout::IntegerExt as _;

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
pub use std::env;
pub use std::format;
pub use std::process::Command;
pub use std::string::String;

mod sysroot;
use sysroot::*;

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

mod vtable;

// Imports for `main``

use std::collections::HashMap;

const DEFAULT_ARGS: &[&str] = &[
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

pub fn show_error(msg: &impl std::fmt::Display) -> ! {
    eprintln!("fatal error: {msg}");
    std::process::exit(101) // exit code needed to make ui_test happy
}

#[macro_export]
macro_rules! show_error {
    ($($tt:tt)*) => {crate::show_error(&format_args!($($tt)*)) };
}

pub fn be_rustc(args: &mut Vec<String>) {
    let use_panic_abort = args
        .array_windows::<2>()
        .any(|[first, second]| first == "--crate-name" && second == "panic_abort");

    if use_panic_abort {
        args.insert(0, "-Cpanic=abort".into());
    } else {
        // #FIXME: we should not need to add this
        args.insert(0, "-Cpanic=unwind".into());
    }

    // Invoke the rust compiler
    let status = Command::new("rustc")
        .args(args)
        .env_remove("RUSTC")
        .env_remove("MINIMIZE_BE_RUSTC")
        .status()
        .expect("failed to invoke rustc in custom sysroot build");

    std::process::exit(status.code().unwrap_or(1));
}

fn main() {
    let mut all_args: Vec<String> = env::args().collect();

    all_args.splice(1..1, DEFAULT_ARGS.iter().map(ToString::to_string));
    if std::env::var("MINIMIZE_BE_RUSTC").as_deref() != Ok("sysroot") {
        let sysroot = get_sysroot_dir();
        all_args.insert(1, format!("--sysroot={}", sysroot.display()));
    }

    if (std::env::var_os("MINIMIZE_BE_RUSTC")).is_some() {
        let mut rustc_args: Vec<String> = all_args.into_iter().skip(1).collect();
        return be_rustc(&mut rustc_args);
    }

    let (minimize_args, rustc_args) = split_args(all_args);
    let dump = minimize_args.iter().any(|x| x == "--minimize-dump");

    let sysroot_mode = std::env::var("MINIMIZE_BUILD_SYSROOT").ok();
    match sysroot_mode.as_deref() {
        Some("only") => {
            setup_sysroot();
            std::process::exit(0);
        }
        Some("off") => {
            // Don't build the sysroot here
        }
        _ => {
            setup_sysroot();
        }
    }

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
                TerminationInfo::Abort => show_error!("program aborted"),
                TerminationInfo::Ub(err) => show_error!("UB: {}", err.get_internal()),
                TerminationInfo::Deadlock => show_error!("program dead-locked"),
                TerminationInfo::MemoryLeak => show_error!("program leaked memory"),
            }
        }
    });
}

/// split arguments into arguments for minimize and rustc
fn split_args(args: Vec<String>) -> (Vec<String>, Vec<String>) {
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

fn get_mini(args: Vec<String>, callback: impl FnOnce(rs::TyCtxt<'_>, Program) + Send + Copy) {
    rustc_driver::run_compiler(&args, &mut Cb { callback });
}

struct Cb<F: FnOnce(rs::TyCtxt<'_>, Program) + Send + Copy> {
    callback: F,
}

impl<F: FnOnce(rs::TyCtxt<'_>, Program) + Send + Copy> rustc_driver::Callbacks for Cb<F> {
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &rustc_interface::interface::Compiler,
        tcx: rs::TyCtxt<'tcx>,
    ) -> rustc_driver::Compilation {
        // StableMIR can only be used inside a `run` call, to guarantee its context is properly
        // initialized. Calls to StableMIR functions will panic if done outside a run.
        let prog = smir::run(tcx, || Ctxt::new(tcx).translate()).unwrap();
        (self.callback)(tcx, prog);
        rustc_driver::Compilation::Stop
    }
}
