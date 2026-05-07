//! This module provides utilities for the terminal user-interface.
//! That is, it bundles common command line argument parsing and error message rendering.

use minirust_rs::{lang::Program, prelude::TerminationInfo};

use crate::{BasicMem, TreeBorrowMem, run::run_program};

pub fn show_error(msg: &impl std::fmt::Display) -> ! {
    eprintln!("fatal error: {msg}");
    std::process::exit(101) // exit code needed to make ui_test happy
}

#[macro_export]
macro_rules! show_error {
    ($($tt:tt)*) => {$crate::cli::show_error(&format_args!($($tt)*)) };
}

pub struct MinirustMachineConfig {
    tree_borrows: bool,
}

impl Default for MinirustMachineConfig {
    fn default() -> Self {
        Self { tree_borrows: false }
    }
}

impl MinirustMachineConfig {
    /// Returns `true` if this was a "minirun" argument, i.e. if it started with `--minirust-`
    pub fn consume_arg(&mut self, arg: &str) -> bool {
        let Some(arg) = arg.strip_prefix("--minirust-") else {
            return false;
        };
        match arg {
            "tree-borrows" => self.tree_borrows = true,
            _ => show_error!("Unknown argument --minirust-{arg}!"),
        }
        true
    }

    /// Runs the program using [`run_program`]. The memory model/machine is constructed according to this config.
    pub fn run_prog(&self, prog: Program) -> TerminationInfo {
        if self.tree_borrows {
            run_program::<TreeBorrowMem>(prog)
        } else {
            run_program::<BasicMem>(prog)
        }
    }

    /// Runs a MiniRust program, see [`Self::run_prog`].
    /// This function returns if the program terminates cleanly, otherwise it prints a
    /// human-readable error message to stderr and exists the process (!) with error code 101.
    pub fn run_prog_and_print_errors(&self, prog: Program) {
        match self.run_prog(prog) {
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
}
