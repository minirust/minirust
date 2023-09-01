use crate::*;

#[test]
fn return_success() {
    let other_f = {
        let locals = [<()>::get_ptype()];
        let b0 = block!(return_());

        function(Ret::Yes, 0, &locals, &[b0])
    };

    let locals = [<()>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        call(1, &[], local(0), Some(1))
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f, other_f]);
    dump_program(p);
    assert_stop(p);
}

#[test]
fn return_no_next() {
    let other_f = {
        let locals = [<()>::get_ptype()];
        let b0 = block!(return_());

        function(Ret::Yes, 0, &locals, &[b0])
    };

    let locals = [<()>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        call(1, &[], local(0), None)
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f, other_f]);
    dump_program(p);
    assert_ub(p, "return from a function where caller did not specify next block");
}


#[test]
fn return_intrinsic_no_next() {
    let locals = [<*const i32>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::Allocate,
            arguments: list![const_int::<usize>(4), const_int::<usize>(4)],
            ret: local(0),
            next_block: None,
        }
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "return from an intrinsic where caller did not specify next block");
}

