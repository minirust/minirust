use crate::*;

fn dummy_function() -> Function {
    let locals = [<()>::get_ptype()];
    let b0 = block!(exit());
    function(Ret::No, 0, &locals, &[b0])
}

#[test]
fn spawn_success() {
    let locals = [ <u32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        spawn(fn_ptr(1), Some(local(0)), 1),
    );
    let b1 = block!(
        join(load(local(0)), 2),
    );
    let b2 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);

    let p = program(&[f, dummy_function()]);
    assert_stop(p);
}

// UB

#[test]
fn spawn_arg_count() {
    let b0 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Spawn,
            arguments: list![],
            ret: None,
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &[], &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid number of arguments for `Intrinsic::Spawn`")
}



#[test]
fn spawn_arg_value() {
    let locals = [<u32>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        assign(local(0), const_int::<u32>(0)),
        spawn(load(local(0)), None, 1),
    );
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid first argument to `Intrinsic::Spawn`")
}


fn takes_args() -> Function {
    let locals = [<()>::get_ptype()];
    let b0 = block!(exit());
    function(Ret::No, 1, &locals, &[b0])
}

#[test]
fn spawn_func_takes_args() {
    let b0 = block!(
        spawn(fn_ptr(1), None, 1),
    );
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &[], &[b0, b1]);

    let p = program(&[f, takes_args()]);
    assert_ub(p, "invalid first argument to `Intrinsic::Spawn`, function takes arguments")
}


fn returns() -> Function {
    let locals = [<()>::get_ptype()];
    let b0 = block!(exit());
    function(Ret::Yes, 0, &locals, &[b0])
}

#[test]
fn spawn_func_returns() {
    let locals = [<()>::get_ptype()];

    let b0 = block!(
        spawn(fn_ptr(1), None, 1),
    );
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f, returns()]);
    assert_ub(p, "invalid first argument to `Intrinsic::Spawn`, function returns something")
}

#[test]
fn spawn_wrongreturn() {
    let locals = [ <()>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        spawn(fn_ptr(1), Some(local(0)), 1),
    );
    let b1 = block!(
        join(load(local(0)), 2),
    );
    let b2 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);

    let p = program(&[f, dummy_function()]);
    assert_ub(p, "invalid return type for `Intrinsic::Spawn`");
}
