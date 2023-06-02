use crate::*;

use gen_minirust::prelude::NdResult;

// Run the program and return its TerminationInfo.
// We fix `BasicMemory` as a memory for now.
pub fn run_program(prog: Program) -> TerminationInfo {
    let res: NdResult<!> = try {
        let mut machine = Machine::<BasicMemory>::new(prog)?;

        loop {
            machine.step()?;

            // Drops everything not reachable from `machine`.
            gen_minirust::libspecr::hidden::mark_and_sweep(&machine);
        }
    };

    // Extract the TerminationInfo from the `NdResult<!>`.
    let res: Result<!, TerminationInfo> = res.get_internal();
    match res {
        Ok(never) => never,
        Err(t) => t,
    }
}
