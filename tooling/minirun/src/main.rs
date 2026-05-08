use std::{env, fs::File, io, ops::Deref};

use miniutil::{cli::MinirustMachineConfig, json, show_error};

fn main() {
    let mut all_args: Vec<String> = env::args().skip(1).collect();

    let mut config = MinirustMachineConfig::default();
    all_args.extract_if(.., |arg| config.consume_arg(arg)).for_each(drop);

    let prog = match all_args.deref() {
        // Note that this only finishes parsing once stdin is closed (by e.g. pressing Ctrl+D when typing interactively).
        [] => json::load_program(io::stdin()),
        [filename] => json::load_program(File::open(filename).expect("Could not open JSON file!")),
        [_, _, ..] => show_error!("Can not load JSON from more than one file!"),
    };

    config.run_prog_and_print_errors(prog);
}
