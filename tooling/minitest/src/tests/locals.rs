use crate::*;

#[test]
fn dead_before_live() {
    let locals = vec![<bool>::get_type()];
    let stmts = vec![storage_dead(0)];
    let p = small_program(&locals, &stmts);
    assert_stop::<BasicMem>(p);
}

#[test]
fn double_live() {
    let locals = vec![<bool>::get_type()];
    let stmts = vec![storage_live(0), storage_live(0)];
    let p = small_program(&locals, &stmts);
    assert_stop::<BasicMem>(p);
}

#[test]
fn assign_dead() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let var = f.declare_local::<u32>();
    f.assign(var, const_int(42u32));
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_ub::<BasicMem>(p, "access to a dead local");
}
