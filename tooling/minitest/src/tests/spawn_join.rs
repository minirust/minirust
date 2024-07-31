use crate::*;

fn dummy_function() -> Function {
    let locals = [<*const ()>::get_type()];
    let b0 = block!(exit());
    function(Ret::No, 1, &locals, &[b0])
}

#[test]
fn spawn_success() {
    let locals = [<u32>::get_type()];

    let b0 = block!(storage_live(0), spawn(fn_ptr_internal(1), null(), local(0), 1),);
    let b1 = block!(join(load(local(0)), 2),);
    let b2 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);

    let p = program(&[f, dummy_function()]);
    assert_stop::<BasicMem>(p);
}

/// The program written out:
/// fn main() {
///     let dp = allocate(sizeof *const ());
///     *dp = dp;
///     let id = spawn(second, *dp);
///     join(id);
/// }
///
/// fn second(dp) {
///     *dp = null;
/// }
///
/// This program should obviously not have a data race, but since
/// we do a trace based search it could have one. This is the reason we track synchronized threads.
#[test]
fn thread_spawn_spurious_race() {
    let pp_ptype = <*const *const ()>::get_type(); // Pointer pointer place type.
    let locals = [pp_ptype, <u32>::get_type()];

    let size = const_int_typed::<usize>(<*const ()>::get_size().unwrap_size().bytes());
    let align = const_int_typed::<usize>(<*const ()>::get_align().bytes());

    let b0 = block!(storage_live(0), allocate(size, align, local(0), 1));
    let b1 = block!(
        storage_live(1),
        assign(deref(load(local(0)), pp_ptype), load(local(0))),
        spawn(fn_ptr_internal(1), load(deref(load(local(0)), pp_ptype)), local(1), 2)
    );
    let b2 = block!(join(load(local(1)), 3));
    let b3 = block!(deallocate(load(local(0)), size, align, 4,));
    let b4 = block!(exit());
    let main = function(Ret::No, 0, &locals, &[b0, b1, b2, b3, b4]);

    let locals = [<()>::get_type(), pp_ptype];
    let b0 = block!(assign(deref(load(local(1)), pp_ptype), null()), return_(),);
    let second = function(Ret::Yes, 1, &locals, &[b0]);

    let prog = program(&[main, second]);

    assert_stop::<BasicMem>(prog);
}

// UB

#[test]
fn spawn_arg_count() {
    let b0 = block!(Terminator::Intrinsic {
        intrinsic: IntrinsicOp::Spawn,
        arguments: list![],
        ret: zst_place(),
        next_block: Some(BbName(Name::from_internal(1))),
    });
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &[], &[b0, b1]);

    let p = program(&[f]);
    assert_ub::<BasicMem>(p, "invalid number of arguments for `Spawn` intrinsic")
}

#[test]
fn spawn_arg_value() {
    let locals = [<u32>::get_type()];

    let b0 = block!(
        storage_live(0),
        assign(local(0), const_int::<u32>(0)),
        spawn(load(local(0)), null(), zst_place(), 1),
    );
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f]);
    assert_ub::<BasicMem>(p, "invalid first argument to `Spawn` intrinsic: not a pointer")
}

fn no_args() -> Function {
    let locals = [];
    let b0 = block!(exit());
    function(Ret::No, 0, &locals, &[b0])
}

#[test]
fn spawn_func_no_args() {
    let locals = [<i32>::get_type()];
    let b0 = block!(storage_live(0), spawn(fn_ptr_internal(1), null(), local(0), 1),);
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f, no_args()]);
    assert_ub::<BasicMem>(p, "call ABI violation: number of arguments does not agree")
}

fn returns() -> Function {
    let locals = [<u32>::get_type(), <*const ()>::get_type()];
    let b0 = block!(assign(local(0), const_int::<u32>(0)), return_());
    function(Ret::Yes, 1, &locals, &[b0])
}

#[test]
fn spawn_func_returns() {
    let locals = [<i32>::get_type()];

    let b0 = block!(storage_live(0), spawn(fn_ptr_internal(1), null(), local(0), 1),);
    let b1 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1]);

    let p = program(&[f, returns()]);
    assert_ub::<BasicMem>(p, "call ABI violation: return types are not compatible")
}

#[test]
fn spawn_wrongreturn() {
    let locals = [<()>::get_type()];

    let b0 = block!(storage_live(0), spawn(fn_ptr_internal(1), null(), local(0), 1),);
    let b1 = block!(join(load(local(0)), 2),);
    let b2 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);

    let p = program(&[f, dummy_function()]);
    assert_ub::<BasicMem>(p, "invalid return type for `Spawn` intrinsic");
}

#[test]
fn spawn_data_ptr() {
    let locals = [<()>::get_type()];

    let b0 =
        block!(storage_live(0), spawn(fn_ptr_internal(1), const_int::<usize>(0), zst_place(), 1),);
    let b1 = block!(join(load(local(0)), 2),);
    let b2 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);

    let p = program(&[f, dummy_function()]);
    assert_ub::<BasicMem>(p, "invalid second argument to `Spawn` intrinsic: not a pointer");
}

fn wrongarg() -> Function {
    let locals = [<()>::get_type()];
    let b0 = block!(exit());
    function(Ret::No, 1, &locals, &[b0])
}

#[test]
fn spawn_wrongarg() {
    let locals = [<u32>::get_type()];

    let b0 = block!(storage_live(0), spawn(fn_ptr_internal(1), null(), local(0), 1),);
    let b1 = block!(join(load(local(0)), 2),);
    let b2 = block!(exit());
    let f = function(Ret::No, 0, &locals, &[b0, b1, b2]);

    let p = program(&[f, wrongarg()]);
    assert_ub::<BasicMem>(p, "call ABI violation: argument types are not compatible");
}

#[test]
fn join_arg_count() {
    let locals = [<()>::get_type()];

    let b0 = block!(Terminator::Intrinsic {
        intrinsic: IntrinsicOp::Join,
        arguments: list!(),
        ret: zst_place(),
        next_block: Some(BbName(Name::from_internal(1)))
    });
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);

    assert_ub::<BasicMem>(p, "invalid number of arguments for `Join` intrinsic");
}

#[test]
fn join_arg_value() {
    let locals = [<()>::get_type()];

    let b0 = block!(storage_live(0), join(load(local(0)), 1),);
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);

    assert_ub::<BasicMem>(p, "invalid first argument to `Join` intrinsic: not an integer");
}

#[test]
fn join_wrongreturn() {
    let locals = [<u32>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::Intrinsic {
            intrinsic: IntrinsicOp::Join,
            arguments: list![const_int::<u32>(1)],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        },
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);

    assert_ub::<BasicMem>(p, "invalid return type for `Join` intrinsic");
}

#[test]
fn join_no_thread() {
    let locals = [<u32>::get_type()];

    let b0 = block!(
        storage_live(0),
        //Valid since the main thread has Id 0.
        assign(local(0), const_int::<u32>(1)),
        join(load(local(0)), 1),
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);

    assert_ub::<BasicMem>(p, "`Join` intrinsic: join non existing thread");
}
