use std::ffi::OsStr;

use std::path::PathBuf;
use std::process::Command;

use rustc_build_sysroot::{BuildMode, SysrootBuilder, SysrootConfig};
use rustc_version::VersionMeta;

fn get_sysroot_dir() -> PathBuf {
    match std::env::var_os("MINIRUST_SYSROOT") {
        Some(dir) => PathBuf::from(dir),
        None => {
            let user_dirs = directories::ProjectDirs::from("org", "rust-lang", "minirust").unwrap();
            user_dirs.cache_dir().to_owned()
        }
    }
}

// Builds (if necessary) and returns a path to the sysroot for our custom MIR compiled libraries
pub fn setup_sysroot() -> PathBuf {
    // Determine where to put the sysroot.
    let sysroot_dir = get_sysroot_dir();

    // Determine where the rust sources are located.
    let rust_src = {
        // Check for `rust-src` rustup component.
        let rustup_src = rustc_build_sysroot::rustc_sysroot_src(Command::new("rustc"))
            .expect("could not determine sysroot source directory");
        if !rustup_src.exists() {
            crate::show_error!("`rust-src` not found");
        }
        rustup_src
    };
    if !rust_src.exists() {
        crate::show_error!("given Rust source directory `{}` does not exist.", rust_src.display());
    }
    if rust_src.file_name().and_then(OsStr::to_str) != Some("library") {
        crate::show_error!(
            "given Rust source directory `{}` does not seem to be the `library` subdirectory of \
             a Rust source checkout.",
            rust_src.display()
        );
    }

    let target = std::env::args()
        .collect::<Vec<_>>()
        .array_windows::<2>()
        .find_map(|[first, second]| (first == "--target").then(|| second.clone()))
        .unwrap_or_else(|| rustc_version::version_meta().expect("rustc").host);

    let sysroot_config = SysrootConfig::WithStd {
        std_features: ["panic-unwind", "backtrace"].into_iter().map(Into::into).collect(),
    };

    // Get this binary to point at as the rustc
    let this_exe = std::env::current_exe().expect("current_exe - minimize binary not found");

    // Probe ourselves for our version
    let mut version_command = Command::new(&this_exe);
    version_command.env("MINIMIZE_BUILD_SYSROOT", "off");
    version_command.env("MINIMIZE_BE_RUSTC", "1");

    let version = VersionMeta::for_command(version_command).unwrap();


    // We want to act as rustc
    let mut cargo_command = Command::new("cargo");
    cargo_command.env("RUSTC", &this_exe);
    cargo_command.env("MINIMIZE_BE_RUSTC", "1");
    cargo_command.env("MINIMIZE_BUILD_SYSROOT", "off");

    // In the sysroot build, set rustc_version()

    // Do the build.
    SysrootBuilder::new(&sysroot_dir, &target)
        .build_mode(BuildMode::Check)
        .sysroot_config(sysroot_config)
        .rustc_version(version)
        .cargo(cargo_command)
        .build_from_source(&rust_src)
        .expect("sysroot build failed");

    sysroot_dir
}
