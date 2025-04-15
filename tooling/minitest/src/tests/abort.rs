use crate::*;

/// Test a simple program that immediately aborts.
#[test]
fn abort() {
    let mut prog = ProgramBuilder::new();

    let mut start = prog.declare_function();
    start.abort();
    let start = prog.finish_function(start);

    let prog = prog.finish_program(start);
    assert_abort::<BasicMem>(prog);
}
