fn cfg(path: &str) -> ui_test::Config {
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
        root_dir: std::path::PathBuf::from(path),
        mode: ui_test::Mode::Pass,
        program: std::path::PathBuf::from("./target/debug/minimize"),
        output_conflict_handling,
        path_filter: Vec::new(),
        dependencies_crate_manifest_path: None,
        dependency_builder: ui_test::DependencyBuilder::default(),
        quiet: false,
        num_test_threads: std::thread::available_parallelism().unwrap(),
    }
}

fn main() {
    ui_test::run_tests(cfg("./tests/pass")).unwrap();
    ui_test::run_tests(cfg("./tests/ub")).unwrap();
}
