use crate::*;

fn other_f() -> Function {
    let locals = [<()>::get_ptype(); 2];
    let b0 = block!(exit());

    function(Ret::Yes, 1, &locals, &[b0])
}

#[test]
fn call_success() {
    let locals = [<()>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        Terminator::Call {
            callee: fn_ptr(1),
            arguments: list![by_value::<()>(const_unit())],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f, other_f()]);
    dump_program(p);
    assert_stop(p);
}

#[test]
fn call_non_exist() {
    let locals = [<()>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        Terminator::Call {
            callee: fn_ptr(1),
            arguments: list![by_value::<()>(const_unit())],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    dump_program(p);
    assert_ill_formed(p);
}

#[test]
fn call_arg_count() {
    let locals = [<()>::get_ptype()];

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
    assert_ub(p, "call ABI violation: number of arguments does not agree");
}

#[test]
fn call_arg_abi() {
    let locals = [<()>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        Terminator::Call {
            callee: fn_ptr(1),
            arguments: list![by_value::<i32>(const_int::<i32>(42))],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f, other_f()]);
    dump_program(p);
    assert_ub(p, "call ABI violation: argument types are not compatible");
}

#[test]
fn call_ret_abi() {
    let locals = [<i32>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        Terminator::Call {
            callee: fn_ptr(1),
            arguments: list![by_value::<()>(const_unit())],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f, other_f()]);
    dump_program(p);
    assert_ub(p, "call ABI violation: return types are not compatible");
}
