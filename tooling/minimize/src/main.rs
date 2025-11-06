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
use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;
pub use std::format;
use std::path::PathBuf;
use std::process::Command;
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

mod vtable;

// Imports for `main``

use rustc_build_sysroot::{BuildMode, SysrootBuilder, SysrootConfig};
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

pub fn get_sysroot_dir() -> PathBuf {
    match std::env::var_os("MINIRUST_SYSROOT") {
        Some(dir) => PathBuf::from(dir),
        None => {
            let user_dirs = directories::ProjectDirs::from("org", "rust-lang", "minirust").unwrap();
            user_dirs.cache_dir().to_owned()
        }
    }
}

fn be_rustc() {
    // Get the rest of the command line arguments
    let mut args: Vec<OsString> = env::args_os().skip(1).collect();

    let use_panic_abort = args
        .array_windows::<2>()
        .any(|[first, second]| first == "--crate-name" && second == "panic_abort");

    // Inject the Rust flags
    for arg in DEFAULT_ARGS {
        args.push(arg.into());
    }

    // If we are building dependencies, inject the sysroot flag
    if std::env::var_os("MINIMIZE_BUILD_DEPS").is_some() {
        let sysroot_dir = get_sysroot_dir();
        args.push(format!("--sysroot={}", sysroot_dir.display()).into());
    }

    args.push("-C".into());

    if use_panic_abort {
        args.push("panic=abort".into());
    } else {
        args.push("panic=unwind".into());
    }

    // Invoke the rust compiler
    let status = Command::new("rustc")
        .args(args)
        .env_remove("RUSTC")
        .env_remove("MINIMIZE_BE_RUSTC")
        .env_remove("MINIMIZE_BUILD_DEPS")
        .status()
        .expect("failed to invoke rustc in custom sysroot build");

    std::process::exit(status.code().unwrap_or(1));
}

fn setup_sysroot() -> PathBuf {
    // Determine where to put the sysroot.
    let sysroot_dir = get_sysroot_dir();
    let sysroot_dir = sysroot_dir.canonicalize().unwrap_or(sysroot_dir); // Absolute path 

    // Determine where the rust sources are located.
    let rust_src = {
        // Check for `rust-src` rustup component.
        let rustup_src = rustc_build_sysroot::rustc_sysroot_src(Command::new("rustc"))
            .expect("could not determine sysroot source directory");
        if !rustup_src.exists() {
            show_error!("`rust-src` not found");
        } else {
            rustup_src
        }
    };
    if !rust_src.exists() {
        show_error!("given Rust source directory `{}` does not exist.", rust_src.display());
    }
    if rust_src.file_name().and_then(OsStr::to_str) != Some("library") {
        show_error!(
            "given Rust source directory `{}` does not seem to be the `library` subdirectory of \
             a Rust source checkout.",
            rust_src.display()
        );
    }

    let mut it = std::env::args();
    let target = it
        .by_ref()
        .position(|a| a == "--target")
        .and_then(|_| it.next())
        .unwrap_or_else(|| rustc_version::version_meta().expect("rustc").host);

    let sysroot_config = SysrootConfig::WithStd {
        std_features: ["panic-unwind", "backtrace"].into_iter().map(Into::into).collect(),
    };

    // Get this binary to point at as the rustc
    let this_exe = std::env::current_exe().expect("current_exe - minimize binary not found");

    // We want to act as rustc
    let mut cargo_command = Command::new("cargo");
    cargo_command.env("RUSTC", &this_exe).env("MINIMIZE_BE_RUSTC", "1");

    // Do the build.
    SysrootBuilder::new(&sysroot_dir, &target)
        .build_mode(BuildMode::Check)
        .sysroot_config(sysroot_config)
        .cargo(cargo_command)
        .build_from_source(&rust_src)
        .expect("sysroot build failed");

    sysroot_dir
}

fn main() {
    if (std::env::var_os("MINIMIZE_BE_RUSTC")).is_some() {
        return be_rustc();
    }

    let (minimize_args, mut rustc_args) = split_args(std::env::args());
    let dump = minimize_args.iter().any(|x| x == "--minimize-dump");

    let sysroot_mode = std::env::var("MINIMIZE_BUILD_SYSROOT").ok();
    let sysroot: PathBuf;
    match sysroot_mode.as_deref() {
        Some("only") => {
            setup_sysroot();
            std::process::exit(0);
        }
        Some("off") => {
            // Don't build the sysroot here
            sysroot = get_sysroot_dir();
            rustc_args.insert(1, format!("--sysroot={}", sysroot.display()));
        }
        _ => {
            sysroot = setup_sysroot();
            rustc_args.insert(1, format!("--sysroot={}", sysroot.display()));
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
