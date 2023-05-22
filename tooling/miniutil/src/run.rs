use crate::{*, mock_write::MockWrite};

use gen_minirust::prelude::NdResult;

// Run the program and return its TerminationInfo.
// We fix `BasicMemory` as a memory for now.
pub fn run_program(prog: Program) -> TerminationInfo {
    let out = std::io::stdout();
    let err = std::io::stderr();

    let res: Result<!, TerminationInfo> = run(prog, out, err);
    match res {
        Ok(never) => never,
        Err(t) => t,
    }
}

// Run the program and return the output as a `Vec<String>` or a termination info if it
// did not terminate correctly.
// We fix `BasicMemory` as a memory for now.
pub fn get_stdout(prog: Program) -> Result<Vec<String>, TerminationInfo> {
    let out = MockWrite::new();
    let err = std::io::stderr();

    let res = run(prog, out.clone(), err);
    match res {
        Ok(never) => never,
        Err(TerminationInfo::MachineStop) => Ok(out.into_strings()),
        Err(info) => Err(info)
    }
}

fn run(prog: Program, stdout: impl GcWrite, stderr: impl GcWrite) -> Result<!, TerminationInfo> {
    let res: NdResult<!> = try {

        let mut machine = Machine::<BasicMemory>::new(prog, DynWrite::new(stdout), DynWrite::new(stderr))?;

        loop {
            machine.step()?;

            // Drops everything not reachable from `machine`.
            gen_minirust::libspecr::hidden::mark_and_sweep(&machine);
        }
    };

    // Extract the TerminationInfo from the `NdResult<!>`.
    res.get_internal()
}
