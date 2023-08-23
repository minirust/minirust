use crate::*;

fn dummy_function() -> Function {
    let locals = [<*const ()>::get_ptype()];
    let b0 = block!(exit());
    function(Ret::No, 1, &locals, &[b0])
}

#[test]
fn spawn_success() {
    let locals = [ <u32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        spawn(fn_ptr(1), null(), local(0), 1),
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
            ret: zst_place(),
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
        spawn(load(local(0)), null(), zst_place(), 1),
    );
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub(p, "invalid first argument to `Intrinsic::Spawn`, not a pointer")
}


fn no_args() -> Function {
    let locals = [];
    let b0 = block!(exit());
    function(Ret::No, 0, &locals, &[b0])
}

#[test]
fn spawn_func_no_args() {
    let locals = [<i32>::get_ptype()];
    let b0 = block!(
        storage_live(0),
        spawn(fn_ptr(1), null(), local(0), 1),
    );
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f, no_args()]);
    assert_ub(p, "call ABI violation: number of arguments does not agree")
}


fn returns() -> Function {
    let locals = [<u32>::get_ptype(), <*const ()>::get_ptype()];
    let b0 = block!(
        assign(local(0), const_int::<u32>(0)),
        return_()
    );
    function(Ret::Yes, 1, &locals, &[b0])
}

#[test]
fn spawn_func_returns() {
    let locals = [<i32>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        spawn(fn_ptr(1), null(), local(0), 1),
    );
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f, returns()]);
    assert_ub(p, "call ABI violation: return types are not compatible")
}

#[test]
fn spawn_wrongreturn() {
    let locals = [ <()>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        spawn(fn_ptr(1), null(), local(0), 1),
    );
    let b1 = block!(
        join(load(local(0)), 2),
    );
    let b2 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);

    let p = program(&[f, dummy_function()]);
    assert_ub(p, "invalid return type for `Intrinsic::Spawn`");
}

#[test]
fn spawn_data_ptr() {
    let locals = [ <()>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        spawn(fn_ptr(1), const_int::<usize>(0), zst_place(), 1),
    );
    let b1 = block!(
        join(load(local(0)), 2),
    );
    let b2 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);

    let p = program(&[f, dummy_function()]);
    assert_ub(p, "invalid second argument to `Intrinsic::Spawn`, not a pointer");
}

fn wrongarg() -> Function {
    let locals = [<()>::get_ptype()];
    let b0 = block!(exit());
    function(Ret::No, 1, &locals, &[b0])
}

#[test]
fn spawn_wrongarg() {
    let locals = [ <u32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        spawn(fn_ptr(1), null(), local(0), 1),
    );
    let b1 = block!(
        join(load(local(0)), 2),
    );
    let b2 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);

    let p = program(&[f, wrongarg()]);
    assert_ub(p, "call ABI violation: argument types are not compatible");
}
