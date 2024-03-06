use crate::*;

#[test]
fn dead_before_live() {
    let locals = vec![<bool>::get_type()];
    let stmts = vec![storage_dead(0)];
    let p = small_program(&locals, &stmts);
    assert_ill_formed(p);
}

#[test]
fn double_live() {
    let locals = vec![<bool>::get_type()];
    let stmts = vec![storage_live(0), storage_live(0)];
    let p = small_program(&locals, &stmts);
    assert_ill_formed(p);
}

#[test]
fn neg_count_array() {
    let ty = array_ty(<()>::get_type(), -1);
    let locals = &[ty];

    let stmts = &[storage_live(0)];

    let p = small_program(locals, stmts);
    dump_program(p);
    assert_ill_formed(p);
}

#[test]
fn no_main() {
    let p = program(&[]);
    assert_ill_formed(p);
}

#[test]
fn too_large_local() {
    let ty = <[u8; usize::MAX / 2 + 1]>::get_type();

    let locals = &[ty];
    let stmts = &[];

    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}

#[test]
fn type_mismatch() {
    let locals = &[<i32>::get_type()];
    let stmts = &[storage_live(0), assign(local(0), const_int::<u32>(0))];
    let p = small_program(locals, stmts);
    assert_ill_formed(p);
}
