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

fn run_tests(mut config: Config) -> Result<()> {
    // We can't use `ui_test::run_tests` as that has bad defaults (it always blesses!).

    // Handle command-line arguments.
    let args = ui_test::Args::test()?;
    if let Format::Pretty = args.format {
        println!("Compiler: {}", config.program.display());
    }

    let bless = std::env::var_os("BLESS").is_some_and(|v| v != "0");
    config.with_args(&args, bless);
    if let OutputConflictHandling::Error(msg) = &mut config.output_conflict_handling {
        *msg = "BLESS=1 ./test.sh".into();
    }

    let text = match args.format {
        Format::Terse => status_emitter::Text::quiet(),
        Format::Pretty => status_emitter::Text::verbose(),
    };
    let name = config.root_dir.display().to_string();
    run_tests_generic(
        vec![config],
        ui_test::default_file_filter,
        ui_test::default_per_file_config,
        (text, status_emitter::Gha::</* GHA Actions groups*/ true> { name }),
    )
}

fn main() -> Result<()> {
    run_tests(cfg("./tests/pass", Mode::Pass))?;
    run_tests(cfg("./tests/ub", Mode::Panic))?;
    Ok(())
}
