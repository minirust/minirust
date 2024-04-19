use std::path::PathBuf;

use ui_test::Mode;

fn cfg(path: &str, mode: Mode) -> ui_test::Config {
    let bless = std::env::var_os("BLESS").is_some_and(|v| v != "0");
    let output_conflict_handling = if bless {
        ui_test::OutputConflictHandling::Bless
    } else {
        ui_test::OutputConflictHandling::Error
    };
    ui_test::Config {
        args: Vec::new(),
        trailing_args: Vec::new(),
        host: Some(String::new()), // not used, ui_test fails if it's not set.
        target: None,
        stderr_filters: Vec::new(),
        stdout_filters: Vec::new(),
        root_dir: PathBuf::from(path),
        mode,
        program: PathBuf::from(env!("CARGO_BIN_EXE_minimize")),
        output_conflict_handling,
        path_filter: Vec::new(),
        dependencies_crate_manifest_path: Some(PathBuf::from("./tests/deps/Cargo.toml")),
        dependency_builder: ui_test::DependencyBuilder::default(),
        quiet: false,
        num_test_threads: std::thread::available_parallelism().unwrap(),
    }
}

fn main() {
    ui_test::run_tests(cfg("./tests/pass", Mode::Pass)).unwrap();
    ui_test::run_tests(cfg("./tests/ub", Mode::Panic)).unwrap();
}
