use crate::*;

#[test]
fn get_metadata_non_ptr_ill_formed() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let x = f.declare_local::<u32>();
        f.storage_live(x);
        f.print(get_metadata(load(x)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "UnOp::GetMetadata: invalid operand: not a pointer");
}

#[test]
fn get_metadata_thin_ptr() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let x = f.declare_local::<u32>();
        let ptr = f.declare_local::<&u32>();
        let nop = f.declare_local::<()>();
        f.storage_live(x);
        f.storage_live(ptr);
        f.storage_live(nop);
        f.assign(ptr, addr_of(x, <&u32>::get_type()));
        f.assign(nop, get_metadata(load(ptr)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}
