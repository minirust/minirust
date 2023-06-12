// This module generates the Mir, and then calls `translate_program` to obtain the `Program`.

use crate::*;
use rustc_driver::{Callbacks, Compilation, RunCompiler};
use rustc_interface::{interface::Compiler, Queries};

pub fn get_mini(file: String, callback: impl FnOnce(Program) + Send + Copy) {
    if !Path::new(&file).exists() {
        eprintln!("You need to define some `file.rs` in order to run `minimize`.");
        std::process::exit(1);
    }

    let args = [
        ".".to_string(),
        file,
        "--sysroot".to_string(),
        sysroot(),
        "-L".to_string(),
        "./intrinsics/target/debug".to_string(),
        "-l".to_string(),
        "intrinsics".to_string(),
        // flags taken from miri (see https://github.com/rust-lang/miri/blob/master/src/lib.rs#L116)
        "-Zalways-encode-mir".to_string(),
        "-Zmir-emit-retag".to_string(),
        "-Zmir-opt-level=0".to_string(),
        "--cfg=miri".to_string(),
        "-Zextra-const-ub-checks".to_string(),
        // miri turns this on.
        // But this generates annoying checked operators containing Asserts.
        "-Cdebug-assertions=off".to_string(),
        // This removes Resume and similar stuff
        "-Cpanic=abort".to_string(),
    ];
    RunCompiler::new(&args, &mut Cb { callback }).run().unwrap();
}

struct Cb<F: FnOnce(Program) + Send + Copy> {
    callback: F,
}

impl<F: FnOnce(Program) + Send + Copy> Callbacks for Cb<F> {
    fn after_analysis<'tcx>(
        &mut self,
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

fn sysroot() -> String {
    let sysroot = std::process::Command::new("rustc")
        .arg("--print=sysroot")
        .current_dir(".")
        .output()
        .unwrap();

    std::str::from_utf8(&sysroot.stdout)
        .unwrap()
        .trim()
        .to_string()
}
