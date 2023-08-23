use crate::*;

#[test]
fn print_success() {
    let locals = [];

    let b0 = block!(
        print(const_int::<u32>(42), 1), // ints can be printed
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    dump_program(p);
    assert_stop(p);
}

#[test]
fn print_fail() {
    let locals = [];

    let b0 = block!(
        print(const_unit(), 1), // tuples cannot be printed
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "unsupported value for printing");
}

#[test]
fn print_wrongreturn() {
    let locals = [<u32>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::PrintStdout,
            arguments: list![const_int::<usize>(4)],
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1))),
        },
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    dump_program(p);
    assert_ub(p, "invalid return type for `Intrinsic::PrintStdout`");
}
