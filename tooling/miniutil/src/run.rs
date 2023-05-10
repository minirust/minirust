use crate::{*, mock_write::MockBuffer};

use gen_minirust::prelude::NdResult;

// Run the program and return its TerminationInfo.
// We fix `BasicMemory` as a memory for now.
pub fn run_program(prog: Program) -> TerminationInfo {
    let out = std::io::stdout();
    let err = std::io::stderr();

    let res: NdResult<!> = try {

        let mut machine = Machine::<BasicMemory>::new(prog, DynWrite::new(out), DynWrite::new(err))?;

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

// Run the program and return the output as a `Vec<String>` or a termination info if it
// did not terminate correctly.
// We fix `BasicMemory` as a memory for now.
pub fn get_out(prog: Program) -> Result<Vec<String>, TerminationInfo> {
    let out = MockBuffer::new();
    let err = std::io::stderr();

    let mut machine = Machine::<BasicMemory>::new(prog, DynWrite::new(out.out()), DynWrite::new(err)).get_internal()?;

    let res: NdResult<!> = try {
        loop {
            machine.step()?;

            // Drops everything not reachable from `machine`.
            gen_minirust::libspecr::hidden::mark_and_sweep(machine);
        }
    };

    let res = res.get_internal();

    match res {
        Ok(never) => never,
        Err(TerminationInfo::MachineStop) => Ok(out.into_strings()),
        Err(info) => Err(info)
    }
}