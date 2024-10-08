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
fn get_thin_non_ptr_ill_formed() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let x = f.declare_local::<u32>();
        f.storage_live(x);
        f.print(get_thin_pointer(load(x)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "UnOp::GetThinPointer: invalid operand: not a pointer");
}

#[test]
fn construct_wide_non_ptr_ill_formed() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let x = f.declare_local::<u32>();
        f.storage_live(x);
        f.print(construct_wide_pointer(load(x), load(x), <&[u32]>::get_type()));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(
        p,
        "BinOp::ConstructWidePointer: invalid left type: not a pointer",
    );
}

#[test]
fn construct_wide_from_wide_ptr_ill_formed() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        // Construct a wide pointer
        let arr = f.declare_local::<[u32; 3]>();
        f.storage_live(arr);
        let arr_ref = addr_of(arr, <&[u32; 3]>::get_type());
        let slice_ref_v = construct_wide_pointer(arr_ref, const_int(3_usize), <&[u32]>::get_type());

        // Try to use the wide ptr to construct another wide ptr
        f.print(construct_wide_pointer(slice_ref_v, const_int(3_usize), <&[u32]>::get_type()));

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(
        p,
        "BinOp::ConstructWidePointer: invalid left type: not a thin pointer",
    );
}

#[test]
fn construct_wide_mismatched_meta_ill_formed() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let x = f.declare_local::<u32>();
        f.storage_live(x);
        f.print(construct_wide_pointer(
            addr_of(x, <&u32>::get_type()),
            const_int(0_u32), // not `usize` as expected
            <&[u32]>::get_type(),
        ));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(
        p,
        "BinOp::ConstructWidePointer: invalid right type: not metadata of target",
    );
}

// PASS below

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

#[test]
fn get_thin_of_thin_ptr() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let x = f.declare_local::<u32>();
        let ptr = f.declare_local::<&u32>();
        let nop = f.declare_local::<*const ()>();
        f.storage_live(x);
        f.storage_live(ptr);
        f.storage_live(nop);
        f.assign(x, const_int(12_u32));
        f.assign(ptr, addr_of(x, <&u32>::get_type()));
        f.assign(nop, get_thin_pointer(load(ptr)));
        // Make sure the new pointer still has the same provenance
        f.assume(eq(load(deref(load(nop), <u32>::get_type())), const_int(12_u32)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn construct_slice() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        // Make array
        let arr = f.declare_local::<[u32; 3]>();
        f.storage_live(arr);
        f.assign(index(arr, const_int(0)), const_int(42_u32));
        f.assign(index(arr, const_int(1)), const_int(43_u32));
        f.assign(index(arr, const_int(2)), const_int(44_u32));
        let arr_ref = addr_of(arr, <&[u32; 3]>::get_type());

        // Construct the slice
        let slice_ref = f.declare_local::<&[u32]>();
        let slice_ref_v = construct_wide_pointer(arr_ref, const_int(3_usize), <&[u32]>::get_type());
        f.storage_live(slice_ref);
        f.assign(slice_ref, slice_ref_v);

        // Assert slice[1] == 43
        let loaded_val = load(index(deref(load(slice_ref), <[u32]>::get_type()), const_int(1)));
        f.assume(eq(loaded_val, const_int(43_u32)));

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn construct_thin() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let x = f.declare_local::<u8>();
        f.storage_live(x);
        f.assign(x, const_int(0xff_u8));
        let cast_ptr =
            construct_wide_pointer(addr_of(x, <&u8>::get_type()), unit(), <&i8>::get_type());

        f.assume(eq(load(deref(cast_ptr, <i8>::get_type())), const_int(-1_i8)));

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}
