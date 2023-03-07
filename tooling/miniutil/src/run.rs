use crate::*;

use std::collections::HashSet;
use GcCompat;

pub fn run_program(prog: Program) -> TerminationInfo {
    fn run_impl(program: Program) -> NdResult<!> {
        let mut machine = Machine::<BasicMemory>::new(program)?;
        mark_and_sweep(&machine);
        loop {
            machine.step()?;
            mark_and_sweep(&machine);
        }
    }

    match run_impl(prog).get() {
        Ok(f) => match f {},
        Err(t) => t,
    }
}

fn mark_and_sweep<M: Memory>(machine: &Machine<M>) {
    let mut set = HashSet::new();
    machine.points_to(&mut set);
    gen_minirust::libspecr::hidden::mark_and_sweep(set);
}
