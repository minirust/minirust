use crate::*;

#[test]
fn assert_true() {
    let mut prog = ProgramBuilder::new();

    let mut start = prog.declare_function();
    start.assert(const_bool(true));
    start.exit();
    let start = prog.finish_function(start);

    let prog = prog.finish_program(start);
    assert_exit(prog);
}

#[test]
fn assert_false() {
    let mut prog = ProgramBuilder::new();

    let mut start = prog.declare_function();
    start.assert(const_bool(false));
    start.exit();
    let start = prog.finish_function(start);

    let prog = prog.finish_program(start);
    assert_panic(prog);
}

#[test]
fn assert_wrong_argty() {
    let mut prog = ProgramBuilder::new();

    let mut start = prog.declare_function();
    start.assert(const_int(0));
    start.exit();
    let start = prog.finish_function(start);

    let prog = prog.finish_program(start);
    assert_ill_formed(prog, "Terminator::Assert: invalid type");
}
