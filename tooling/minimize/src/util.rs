use std::env;
use std::ffi::OsStr;
use std::ffi::OsString;

use std::path::PathBuf;
use std::process::Command;

use rustc_build_sysroot::{BuildMode, SysrootBuilder, SysrootConfig};

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

pub fn show_error(msg: &impl std::fmt::Display) -> ! {
    eprintln!("fatal error: {msg}");
    std::process::exit(101) // exit code needed to make ui_test happy
}

#[macro_export]
macro_rules! show_error {
    ($($tt:tt)*) => {show_error(&format_args!($($tt)*)) };
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

pub fn be_rustc() {
    // Get the rest of the command line arguments
    let mut args: Vec<OsString> = env::args_os().skip(1).collect();

    let use_panic_abort = args
        .array_windows::<2>()
        .any(|[first, second]| first == "--crate-name" && second == "panic_abort");

    // Inject the Rust flags
    for arg in DEFAULT_ARGS {
        args.push(arg.into());
    }

    // Inject the sysroot flag unless we are building the sysroot itself
    if std::env::var("MINIMIZE_BE_RUSTC").as_deref() != Ok("sysroot") {
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
        .status()
        .expect("failed to invoke rustc in custom sysroot build");

    std::process::exit(status.code().unwrap_or(1));
}

// Builds (if necessary) and returns a path to the sysroot for our custom MIR compiled libraries
pub fn setup_sysroot() -> PathBuf {
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
    cargo_command.env("RUSTC", &this_exe).env("MINIMIZE_BE_RUSTC", "sysroot");

    // Do the build.
    SysrootBuilder::new(&sysroot_dir, &target)
        .build_mode(BuildMode::Check)
        .sysroot_config(sysroot_config)
        .cargo(cargo_command)
        .build_from_source(&rust_src)
        .expect("sysroot build failed");

    sysroot_dir
}
