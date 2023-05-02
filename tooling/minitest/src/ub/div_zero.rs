use crate::*;

#[test]
fn div_zero() {
    let locals = [<i32>::get_ptype()];

    let b0 = block!(
        storage_live(0),
        assign(
            local(0),
            div::<i32>(
                const_int::<i32>(1),
                const_int::<i32>(0),
            )
        ),
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f], &[]);
    dump_program(p);
    assert_ub(p, "division by zero");
}
