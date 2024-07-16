use crate::*;

fn other_f() -> Function {
    let locals = [<()>::get_type(); 2];
    let b0 = block!(exit());

    function(Ret::Yes, 1, &locals, &[b0])
}

#[test]
fn call_success() {
    let locals = [<()>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::Call {
            callee: fn_ptr(1),
            arguments: list![by_value(unit())],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f, other_f()]);
    dump_program(p);
    assert_stop::<BasicMem>(p);
}

#[test]
fn call_non_exist() {
    let locals = [<()>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::Call {
            callee: fn_ptr(1),
            arguments: list![by_value(unit())],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    dump_program(p);
    assert_ill_formed::<BasicMem>(p, "Constant::FnPointer: invalid function name");
}

#[test]
fn call_arg_count() {
    let locals = [<()>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::Call {
            callee: fn_ptr(1),
            arguments: list![],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f, other_f()]);
    dump_program(p);
    assert_ub::<BasicMem>(p, "call ABI violation: number of arguments does not agree");
}

#[test]
fn call_arg_abi() {
    let locals = [<()>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::Call {
            callee: fn_ptr(1),
            arguments: list![by_value(const_int::<i32>(42))],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f, other_f()]);
    dump_program(p);
    assert_ub::<BasicMem>(p, "call ABI violation: argument types are not compatible");
}

#[test]
fn call_ret_abi() {
    let locals = [<i32>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::Call {
            callee: fn_ptr(1),
            arguments: list![by_value(unit())],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f, other_f()]);
    dump_program(p);
    assert_ub::<BasicMem>(p, "call ABI violation: return types are not compatible");
}

#[test]
fn ret_mismatch() {
    // function that returns a u64
    let other_f = {
        let locals = [<u64>::get_type()];
        let b0 = block!(assign(local(0), const_int::<u64>(0)), return_());

        function(Ret::Yes, 0, &locals, &[b0])
    };

    let locals = [<u8>::get_type()];

    let b0 = block!(
        storage_live(0),
        // call to the function with a u8 place to put the value into.
        call(1, &[], local(0), Some(1))
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f, other_f]);
    dump_program(p);
    assert_ub::<BasicMem>(p, "call ABI violation: return types are not compatible");
}
