use crate::*;

#[test]
fn place_mention_dangling_pointer() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let var = f.declare_local::<*const u32>();
    f.storage_live(var);
    f.assign(var, transmute(const_int(16usize), <*const u32>::get_type()));
    f.place_mention(deref(load(var), <i32>::get_type())); // `let _ = *var;`
    f.storage_dead(var);
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn place_mention_not_ignored() {
    // It is still UB if the place expression itself causes UB.
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let var = f.declare_local::<*const *const u32>();
    f.storage_live(var);
    f.assign(var, transmute(const_int(16usize), <*const *const u32>::get_type()));
    f.place_mention(deref(load(deref(load(var), <*const i32>::get_type())), <i32>::get_type())); // `let _ = *var;`
    f.storage_dead(var);
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_ub::<BasicMem>(p, "dereferencing pointer without provenance");
}
