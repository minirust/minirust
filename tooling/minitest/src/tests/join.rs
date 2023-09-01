use crate::*;

fn dummy_function() -> Function {
    let locals = [<*const ()>::get_type()];

    let b0 = block!(exit());

    function(Ret::No, 1, &locals, &[b0])
}

// Duplication of `spawn::spawn_success` for consistency.
#[test]
fn join_success() {
    let locals = [ <u32>::get_type() ];

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

#[test]
fn join_arg_count() {
    let locals = [ <()>::get_type() ];

    let b0 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Join,
            arguments: list!(),
            ret: zst_place(),
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);

    assert_ub(p, "invalid number of arguments for `Intrinsic::Join`");
}

#[test]
fn join_arg_value() {
    let locals = [ <()>::get_type() ];

    let b0 = block!(
        storage_live(0),
        join(load(local(0)), 1),
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);

    assert_ub(p, "invalid first argument to `Intrinsic::Join`, not an integer");
}

#[test]
fn join_wrongreturn() {
    let locals = [ <u32>::get_type() ];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Join,
            arguments: list![const_int::<u32>(1)],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        },
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);

    assert_ub(p, "invalid return type for `Intrinsic::Join`");
}

#[test]
fn join_no_thread() {
    let locals = [ <u32>::get_type() ];

    let b0 = block!(
        storage_live(0),
        //Valid since the main thread has Id 0.
        assign(local(0), const_int::<u32>(1)),
        join(load(local(0)), 1),
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);

    assert_ub(p, "`Intrinsic::Join`: join non existing thread");
}
