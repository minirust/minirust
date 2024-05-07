use std::path::PathBuf;

use ui_test::color_eyre::eyre::Result;
use ui_test::{
    run_tests_generic, status_emitter, CommandBuilder, Config, Format, Mode, OutputConflictHandling,
};

fn cfg(path: &str, mode: Mode) -> Config {
    let mut program = CommandBuilder::rustc();
    program.program = PathBuf::from(env!("CARGO_BIN_EXE_minimize"));
    Config {
        mode,
        program,
        dependencies_crate_manifest_path: Some(PathBuf::from("./tests/deps/Cargo.toml")),
        out_dir: PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ui"),
        ..Config::rustc(path)
    }
}

fn run_tests(mut configs: Vec<Config>) -> Result<()> {
    // Some of this is adapted from `ui_test::run_tests`.
    // Handle command-line arguments.
    let args = ui_test::Args::test()?;
    let bless = std::env::var_os("BLESS").is_some_and(|v| v != "0");

    for config in configs.iter_mut() {
        config.with_args(&args, bless);
        if let OutputConflictHandling::Error(msg) = &mut config.output_conflict_handling {
            *msg = "BLESS=1 ./test.sh".into();
        }
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
    run_tests(vec![cfg("tests/pass", Mode::Pass), cfg("tests/ub", Mode::Panic)])
}
