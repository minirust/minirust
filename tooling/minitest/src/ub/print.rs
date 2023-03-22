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
