use std::path::PathBuf;
use std::process::Command;

use ui_test::color_eyre::eyre::Result;
use ui_test::dependencies::DependencyBuilder;
use ui_test::spanned::Spanned;
use ui_test::{
    CommandBuilder, Config, Format, OutputConflictHandling, run_tests_generic, status_emitter,
};

enum Mode {
    Pass,
    Panic,
}

fn cfg(path: &str, mode: Mode) -> Config {
    let mut program = CommandBuilder::rustc();
    program.program = PathBuf::from(env!("CARGO_BIN_EXE_minimize"));

    let mut config = Config {
        program,
        out_dir: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ui"),
        ..Config::rustc(path)
    };

    let exit_status = match mode {
        Mode::Pass => 0,
        Mode::Panic => 101,
    };
    let require_annotations = false; // we're not showing errors in a specifc line anyway
    config.comment_defaults.base().exit_status = Spanned::dummy(exit_status).into();
    config.comment_defaults.base().require_annotations = Spanned::dummy(require_annotations).into();

    let mut dependency_program = CommandBuilder::cargo();

    // Change cargo command from `build` to `check`.
    dependency_program.args[0] = "check".into();

    // Get the minimize binary to point at as the rustc
    let minimize_exe = PathBuf::from(env!("CARGO_BIN_EXE_minimize"));

    dependency_program.envs.push(("RUSTC".into(), Some(minimize_exe.into())));
    dependency_program.envs.push(("MINIMIZE_BE_RUSTC".into(), Some("deps".into())));
    dependency_program.envs.push(("MINIMIZE_BUILD_SYSROOT".into(), Some("off".into())));

    // To let tests use dependencies, we have to add a `DependencyBuilder`
    // custom "comment" (with arbitrary name), which will then take care
    // of building the dependencies and making them available in the test.
    config.comment_defaults.base().set_custom(
        "dependencies",
        DependencyBuilder {
            program: dependency_program,
            crate_manifest_path: "./tests/deps/Cargo.toml".into(),
            ..Default::default()
        },
    );
    config
}

fn run_tests(mut configs: Vec<Config>) -> Result<()> {
    // Some of this is adapted from `ui_test::run_tests`.
    // Handle command-line arguments.
    let args = ui_test::Args::test()?;
    let bless = std::env::var_os("BLESS").is_some_and(|v| v != "0");

    for config in configs.iter_mut() {
        config.with_args(&args);
        if bless {
            config.output_conflict_handling = OutputConflictHandling::Bless;
        }
        config.bless_command = Some("BLESS=1 ./mini test".into());
    }

    let text = match args.format {
        Format::Terse => status_emitter::Text::quiet(),
        Format::Pretty => status_emitter::Text::verbose(),
    };
    run_tests_generic(
        configs,
        ui_test::default_file_filter,
        ui_test::default_per_file_config,
        (text, status_emitter::Gha::</* GHA Actions groups*/ true> { name: format!("minimize") }),
    )
}

fn main() -> Result<()> {
    Command::new(env!("CARGO_BIN_EXE_minimize"))
        .env("MINIMIZE_BUILD_SYSROOT", "only")
        .status()
        .expect("failed to build sysroot for testing");

    run_tests(vec![
        cfg("tests/pass", Mode::Pass),
        cfg("tests/ub", Mode::Panic),
        cfg("tests/panic", Mode::Panic),
    ])
}
