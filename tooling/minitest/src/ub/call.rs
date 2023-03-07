use crate::*;

fn other_f() -> Function {
    let locals = [<()>::get_ptype(); 2];
    let b0 = block2(&[&exit()]);

    function(Ret::Yes, 1, &locals, &[b0])
}

fn other_arg_abi() -> ArgAbi {
    ArgAbi::Stack(Size::ZERO, Align::ONE)
}

#[test]
fn call_success() {
    let locals = [<()>::get_ptype()];

    let b0 = block2(&[
        &live(0),
        &Terminator::Call {
            callee: fn_ptr(1),
            arguments: list![(const_unit(), ArgAbi::Register)],
            ret: Some((local(0), ArgAbi::Register)),
            next_block: Some(BbName(Name::new(1))),
        }
    ]);
    let b1 = block2(&[&exit()]);

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f, other_f()]);
    dump_program(&p);
    assert_stop(p);
}

#[test]
fn call_non_exist() {
    let locals = [<()>::get_ptype()];

    let b0 = block2(&[
        &live(0),
        &Terminator::Call {
            callee: fn_ptr(1),
            arguments: list![(const_unit(), ArgAbi::Register)],
            ret: Some((local(0), ArgAbi::Register)),
            next_block: Some(BbName(Name::new(1))),
        }
    ]);
    let b1 = block2(&[&exit()]);

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    dump_program(&p);
    assert_ill_formed(p);
}

#[test]
fn call_arg_count() {
    let locals = [<()>::get_ptype()];

    let b0 = block2(&[
        &live(0),
        &Terminator::Call {
            callee: fn_ptr(1),
            arguments: list![],
            ret: Some((local(0), ArgAbi::Register)),
            next_block: Some(BbName(Name::new(1))),
        }
    ]);
    let b1 = block2(&[&exit()]);

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f, other_f()]);
    dump_program(&p);
    assert_ub(p, "call ABI violation: number of arguments does not agree");
}

#[test]
fn call_arg_abi() {
    let locals = [<()>::get_ptype()];

    let b0 = block2(&[
        &live(0),
        &Terminator::Call {
            callee: fn_ptr(1),
            arguments: list![(const_unit(), other_arg_abi())],
            ret: Some((local(0), ArgAbi::Register)),
            next_block: Some(BbName(Name::new(1))),
        }
    ]);
    let b1 = block2(&[&exit()]);

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f, other_f()]);
    dump_program(&p);
    assert_ub(p, "call ABI violation: argument ABI does not agree");
}

#[test]
fn call_ret_abi() {
    let locals = [<()>::get_ptype()];

    let b0 = block2(&[
        &live(0),
        &Terminator::Call {
            callee: fn_ptr(1),
            arguments: list![(const_unit(), ArgAbi::Register)],
            ret: Some((local(0), other_arg_abi())),
            next_block: Some(BbName(Name::new(1))),
        }
    ]);
    let b1 = block2(&[&exit()]);

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f, other_f()]);
    dump_program(&p);
    assert_ub(p, "call ABI violation: return ABI does not agree");
}
