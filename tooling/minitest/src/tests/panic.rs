use crate::*;

#[test]
fn panic() {
    let mut prog = ProgramBuilder::new();

    let mut start = prog.declare_function();
    start.panic();
    let start = prog.finish_function(start);

    let prog = prog.finish_program(start);
    assert_abort::<BasicMem>(prog, "we panicked");
}
