#![feature(rustc_private)]
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
extern crate rustc_public;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;

mod rs {
    pub use rustc_abi as abi;
    pub use rustc_abi::{
        Align, CanonAbi, FieldIdx, FieldsShape, Size, TagEncoding, VariantIdx, Variants,
    };
    pub use rustc_const_eval::const_eval::mk_eval_cx_for_const_val;
    pub use rustc_const_eval::interpret::{InterpCx, OpTy};
    pub use rustc_middle::mir::{self, interpret::*, *};
    pub use rustc_middle::span_bug;
    pub use rustc_middle::ty::layout::{FnAbiError, FnAbiRequest, LayoutError, TyAndLayout};
    pub use rustc_middle::ty::*;
    pub use rustc_mir_dataflow::impls::always_storage_live_locals;
    pub use rustc_span::Spanned;
    pub use rustc_span::{DUMMY_SP, Span, sym};
    pub use rustc_target::callconv::FnAbi;

    pub type CompileTimeInterpCx<'tcx> =
        InterpCx<'tcx, rustc_const_eval::const_eval::CompileTimeMachine<'tcx>>;
}

use miniutil::cli::MinirustMachineConfig;
use miniutil::show_error;
// Traits
pub use rustc_abi::HasDataLayout as _;
pub use rustc_middle::ty::layout::IntegerExt as _;

mod smir {
    pub use rustc_public::mir::mono::*;
    pub use rustc_public::mir::*;
    pub use rustc_public::rustc_internal::*;
    pub use rustc_public::ty::*;
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
pub use miniutil::json;
pub use miniutil::pretty;
pub use miniutil::run::*;

// Get back some `std` items
pub use std::format;
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
use std::env;
use std::process::ExitCode;

const DEFAULT_ARGS: &[&str] = &[
    // This is the same as Miri's `MIRI_DEFAULT_ARGS`, ensuring we get a MIR with all the UB still present.
    "--cfg=miri",
    "-Zalways-encode-mir",
    "-Zextra-const-ub-checks",
    "-Zmir-emit-retag",
    "-Zmir-opt-level=0",
    "-Zmir-enable-passes=-CheckAlignment",
    "-Zmir-preserve-ub",
    // Also disable UB checks (since `cfg(miri)` in the standard library do not trigger for us).
    "-Zub-checks=false",
];

pub fn be_rustc(mut args: Vec<String>) {
    struct BeRustcCallbacks;
    impl rustc_driver::Callbacks for BeRustcCallbacks {}

    let use_panic_abort = args
        .array_windows::<2>()
        .any(|[first, second]| first == "--crate-name" && second == "panic_abort");

    if use_panic_abort {
        args.insert(1, "-Cpanic=abort".into());
    }

    let exit_code = rustc_driver::catch_with_exit_code(move || {
        rustc_driver::run_compiler(&args, &mut BeRustcCallbacks)
    });

    std::process::exit(if exit_code == ExitCode::SUCCESS {
        rustc_driver::EXIT_SUCCESS
    } else {
        rustc_driver::EXIT_FAILURE
    });
}

fn main() {
    // Compute the rustc flags we will use. Start by adding our default flags before the
    // user-defined ones.
    let mut all_args: Vec<String> = env::args().collect();
    all_args.splice(1..1, DEFAULT_ARGS.iter().map(ToString::to_string));

    // Do sysroot setup, and add the flag for that.
    let sysroot_mode = std::env::var("MINIMIZE_BUILD_SYSROOT").ok();
    match sysroot_mode.as_deref() {
        Some("only") => {
            setup_sysroot();
            return;
        }
        Some("off") => {}
        // If we are probed for our version as rustc, act like sysroot_mode is off to avoid infinite looping
        _ if all_args.iter().any(|a| a == "-vV" || a.starts_with("--print=")) => {}
        _ => {
            let dir = setup_sysroot();
            all_args.insert(1, format!("--sysroot={}", dir.display()));
        }
    }

    if (std::env::var_os("MINIMIZE_BE_RUSTC")).is_some() {
        return be_rustc(all_args);
    }
    let (config, rustc_args) = parse_args(all_args);

    get_mini(rustc_args, |_tcx, prog| {
        if let Some(ref dump_kind) = config.dump {
            match dump_kind.as_str() {
                "pretty" => pretty::dump_program(prog),
                "json" => json::dump_program(&prog),
                x => show_error!("Unknown dump format {x}"),
            }
        } else {
            if config.check_json_roundtrip {
                json::assert_roundtrip(&prog);
            }
            config.machine_config.run_prog_and_print_errors(prog);
        }
    });
}

struct MinimizeConfig {
    /// The argument of `--minimize-dump=...`, or `None` if this flag was not given.
    dump: Option<String>,
    /// If we should check whether de- and reserializing a program round-trips.
    check_json_roundtrip: bool,
    /// The flags we collected for the machine config.
    machine_config: MinirustMachineConfig,
}

/// split arguments into arguments for minimize, minirun, and rustc
fn parse_args(args: Vec<String>) -> (MinimizeConfig, Vec<String>) {
    let mut config = MinimizeConfig {
        dump: None,
        check_json_roundtrip: false,
        machine_config: MinirustMachineConfig::default(),
    };
    let mut rustc_args: Vec<String> = Vec::new();
    for arg in args {
        if let Some(arg) = arg.strip_prefix("--minimize-") {
            if let Some(arg) = arg.strip_prefix("dump=") {
                if config.dump.is_some() {
                    show_error!("Argument --minimize-dump was given twice!");
                } else {
                    config.dump = Some(arg.to_string())
                }
            } else if arg == "check-json-roundtrip" {
                config.check_json_roundtrip = true;
            } else {
                show_error!("Unknown minimize argument --minimize-{arg}!");
            }
        } else if config.machine_config.consume_arg(&arg) {
            continue;
        } else {
            rustc_args.push(arg);
        }
    }
    (config, rustc_args)
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
