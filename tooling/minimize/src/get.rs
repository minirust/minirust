// This module generates the Mir, and then calls `translate_program` to obtain the `Program`.

use crate::*;
use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_interface::{interface::Compiler, Queries};
use rustc_session::EarlyErrorHandler;

pub const DEFAULT_ARGS: &[&str] = &[
    "--cfg=miri",
    "-Zalways-encode-mir",
    "-Zmir-opt-level=0",
    // This generates annoying checked operators containing Asserts.
    "-Cdebug-assertions=off",
    // This removes Resume and similar stuff.
    "-Cpanic=abort",
];

pub fn get_mini(callback: impl FnOnce(Program) + Send + Copy) {
    let mut args: Vec<_> = std::env::args().collect();
    args.splice(1..1, DEFAULT_ARGS.iter().map(ToString::to_string));
    RunCompiler::new(&args, &mut Cb { callback }).run().unwrap();
}

struct Cb<F: FnOnce(Program) + Send + Copy> {
    callback: F,
}

impl<F: FnOnce(Program) + Send + Copy> Callbacks for Cb<F> {
    fn after_analysis<'tcx>(
        &mut self,
        _handler: &EarlyErrorHandler,
        _compiler: &Compiler,
        queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        queries.global_ctxt().unwrap().enter(|arg| {
            let prog = Ctxt::new(arg).translate();
            (self.callback)(prog);
        });

        Compilation::Stop
    }
}
