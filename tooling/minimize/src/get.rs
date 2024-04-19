// This module generates the Mir, and then calls `translate_program` to obtain the `Program`.

use crate::*;
use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_interface::{interface::Compiler, Queries};

pub const DEFAULT_ARGS: &[&str] = &[
    "--cfg=miri",
    "-Zalways-encode-mir",
    "-Zmir-opt-level=0",
    // This generates annoying checked operators containing Asserts.
    "-Cdebug-assertions=off",
    // This removes Resume and similar stuff.
    "-Cpanic=abort",
];

pub fn get_mini(callback: impl FnOnce(rs::TyCtxt<'_>, Program) + Send + Copy) {
    let mut args: Vec<_> = std::env::args().collect();
    args.splice(1..1, DEFAULT_ARGS.iter().map(ToString::to_string));
    RunCompiler::new(&args, &mut Cb { callback }).run().unwrap();
}

struct Cb<F: FnOnce(rs::TyCtxt<'_>, Program) + Send + Copy> {
    callback: F,
}

impl<F: FnOnce(rs::TyCtxt<'_>, Program) + Send + Copy> Callbacks for Cb<F> {
    fn after_analysis<'tcx>(
        &mut self,
        _compiler: &Compiler,
        queries: &'tcx Queries<'tcx>,
    ) -> Compilation {
        queries.global_ctxt().unwrap().enter(|tcx| {
            // StableMIR can only be used inside a `run` call, to guarantee its context is properly
            // initialized. Calls to StableMIR functions will panic if done outside a run.
            let prog = smir::run(tcx, || Ctxt::new(tcx).translate()).unwrap();
            (self.callback)(tcx, prog);
        });

        Compilation::Stop
    }
}
