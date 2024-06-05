use std::u32;

use crate::*;

#[test]
fn global_var() {
    let mut p = ProgramBuilder::new();
    let var = p.declare_global_zero_initialized::<u32>();

    let mut f = p.declare_function();
    f.assign(var, const_int(42u32));
    f.if_(eq(load(var), const_int(42u32)), |f| f.exit(), |f| f.unreachable());
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop(p);
}

#[test]
fn local_var() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let var = f.declare_local::<u32>();
    f.storage_live(var);
    f.assign(var, const_int(42u32));
    f.if_(eq(load(var), const_int(42u32)), |_| {}, |f| f.unreachable());
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop(p);
}

#[test]
fn arg_and_ret_var() {
    let mut p = ProgramBuilder::new();

    let add_two_if_42: FnName = {
        let mut f = p.declare_function();
        let var = f.declare_arg::<u32>();
        let ret = f.declare_ret::<u32>();
        f.if_(eq(load(var), const_int(42u32)), |_| {}, |f| f.unreachable());
        f.assign(ret, add(load(var), const_int(2u32)));
        f.return_();
        p.finish_function(f)
    };

    let start: FnName = {
        let mut start = p.declare_function();
        let ret_place = start.declare_local::<u32>();
        start.storage_live(ret_place);
        start.call(ret_place, add_two_if_42, &[by_value(const_int(42u32))]);
        start.if_(eq(load(ret_place), const_int(44u32)), |f| f.exit(), |f| f.unreachable());
        p.finish_function(start)
    };

    let p = p.finish_program(start);
    assert_stop(p);
}

#[test]
fn switch_int() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let var = f.declare_local::<u32>();
    f.storage_live(var);
    f.assign(var, const_int(42_u32));
    f.switch_int(
        load(var),
        &[
            (41, &|f| f.assign(var, add(load(var), const_int(0u32)))),
            (42, &|f| f.assign(var, add(load(var), const_int(1u32)))),
            (43, &|f| f.assign(var, add(load(var), const_int(0u32)))),
        ],
        |f| f.assign(var, add(load(var), const_int(0u32))),
    );
    f.if_(eq(load(var), const_int(43_u32)), |_| {}, |f| f.unreachable());
    f.storage_dead(var);
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop(p);
}

#[test]
fn while_() {
    let mut p = ProgramBuilder::new();
    let mut f = p.declare_function();
    let var = f.declare_local::<u32>();
    let counter = f.declare_local::<u32>();
    f.storage_live(var);
    f.storage_live(counter);
    f.assign(var, const_int(0u32));
    f.assign(counter, const_int(0u32));
    f.while_(lt(load(counter), const_int(42u32)), |f| {
        f.assign(counter, add(load(counter), const_int(1u32)));
        f.assign(var, add(load(var), const_int(2u32)));
    });
    f.if_(eq(load(var), const_int(84u32)), |f| f.exit(), |f| f.unreachable());
    let f = p.finish_function(f);
    let p = p.finish_program(f);
    assert_stop(p);
}

#[test]
#[should_panic(expected = "PlaceExpr is not a local")]
fn storage_live_with_non_local() {
    let mut p = ProgramBuilder::new();
    let g = p.declare_global_zero_initialized::<i32>();
    let mut f = p.declare_function();
    f.storage_live(g);
}

#[test]
#[should_panic(expected = "PlaceExpr is not a local")]
fn storage_dead_with_non_local() {
    let mut p = ProgramBuilder::new();
    let g = p.declare_global_zero_initialized::<i32>();

    let mut f = p.declare_function();
    f.storage_dead(g);
}

#[test]
#[should_panic(expected = "finish_block: there is no block to finish")]
fn double_exit() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    f.exit();
    f.exit(); // this is wrong, we already finished the block
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop(p);
}

#[test]
#[should_panic(expected = "There is no current block. Cannot insert statement/terminator.")]
fn statement_after_exit() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let var = f.declare_local::<u32>();
    f.exit();
    f.assign(var, const_int(42_u32)); // this is wrong, we already finished the block
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop(p);
}

#[test]
#[should_panic(
    expected = "Function has an unfinished block. You need to return or exit from the last block."
)]
fn no_exit() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let var = f.declare_local::<u32>();
    f.storage_live(var);
    f.assign(var, const_int(42_u32));
    // Here we are forgetting to finish the block.
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop(p);
}
