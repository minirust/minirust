use crate::*;

/// Asserts wide and thin pointers are ABI incompatible
#[test]
fn ub_wide_thin_abi_incompatible() {
    let mut p = ProgramBuilder::new();

    let foo = {
        let mut f = p.declare_function();
        let _arg = f.declare_arg::<*const [u32]>();
        f.exit();
        p.finish_function(f)
    };

    let main = {
        let mut f = p.declare_function();
        let x = f.declare_local::<u32>();
        f.storage_live(x);
        // UB: `*const u32` and `*const [u32]` are different size and thus certainly
        // not ABI compatible.
        f.call_ignoreret(fn_ptr(foo), &[by_value(addr_of(x, <*const u32>::get_type()))]);
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main);
    assert_ub::<BasicMem>(p, "call ABI violation: argument types are not compatible");
}

/// Asserts GetMetadata only works on pointers
#[test]
fn ill_get_metadata_non_ptr() {
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

/// Asserts GetThinPointer only works on pointers
#[test]
fn ill_get_thin_non_ptr() {
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

/// Asserts ConstructWidePointer only works for pointer types
#[test]
fn ill_construct_wide_non_ptr() {
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

/// Asserts we cannot use a wide pointer as the thin pointer part of a wide pointer
#[test]
fn ill_construct_wide_from_wide_ptr() {
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

/// Asserts that the metadata must match the type when constructing a wide pointer
#[test]
fn ill_construct_wide_mismatched_meta() {
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

/// It is ill-formed to compare a wide pointer to a thin pointer.
#[test]
fn ill_compare_wide_thin_ptr() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let val = f.declare_local::<u32>();
        let w_ptr = f.declare_local::<*const [u32]>();
        let t_ptr = f.declare_local::<*const u32>();
        f.storage_live(val);
        f.storage_live(w_ptr);
        f.storage_live(t_ptr);
        f.assign(t_ptr, addr_of(val, <*const u32>::get_type()));
        f.assign(
            w_ptr,
            construct_wide_pointer(
                addr_of(val, <*const u32>::get_type()),
                const_int(1_usize),
                <*const [u32]>::get_type(),
            ),
        );

        f.assume(eq(load(w_ptr), load(t_ptr)));

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "BinOp::Rel: invalid right type");
}

// PASS below

/// Asserts we can use GetMetadata on thin pointers, which just returns a unit value
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

/// Asserts we can use GetThinPointer also on already thin pointers
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

/// Asserts we can use ConstructWidePointer to construct a slice
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

/// Asserts we can use ConstructWidePointer also to construct thin pointers with unit metadata
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

/// Checks that slice pointers are compared lexicographically in the address and then element count.
#[test]
fn compare_slice_ptr() {
    fn subslice(arr_place: PlaceExpr, idx: usize, len: usize) -> ValueExpr {
        construct_wide_pointer(
            addr_of(index(arr_place, const_int(idx)), <*const u32>::get_type()),
            const_int(len),
            <*const [u32]>::get_type(),
        )
    }

    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let dummy = f.declare_local::<[u32; 4]>();
        f.storage_live(dummy);

        f.assume(eq(subslice(dummy, 0, 1), subslice(dummy, 0, 1)));
        f.assume(gt(subslice(dummy, 1, 1), subslice(dummy, 0, 1)));
        f.assume(lt(subslice(dummy, 0, 1), subslice(dummy, 1, 1)));
        f.assume(lt(subslice(dummy, 0, 2), subslice(dummy, 1, 1)));
        f.assume(lt(subslice(dummy, 0, 1), subslice(dummy, 1, 2)));

        f.assume(lt(subslice(dummy, 0, 1), subslice(dummy, 0, 2)));
        f.assume(gt(subslice(dummy, 0, 2), subslice(dummy, 0, 1)));

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

/// Checks that trait object pointers are compared lexicographically in the address and then the address of the vtable.
#[test]
fn compare_trait_obj_ptr() {
    fn wide_pointer(
        arr_place: PlaceExpr,
        idx: usize,
        vtable: VTableName,
        trait_name: TraitName,
    ) -> ValueExpr {
        construct_wide_pointer(
            addr_of(index(arr_place, const_int(idx)), <*const u32>::get_type()),
            const_vtable(vtable, trait_name),
            raw_ptr_ty(PointerMetaKind::VTablePointer(trait_name)),
        )
    }

    // Create two vtables for the same trait
    let mut p = ProgramBuilder::new();
    let b = p.declare_trait();
    let trait_name = p.finish_trait(b);
    let b = p.declare_vtable_for_ty(trait_name, <u32>::get_type());
    let vtable1 = p.finish_vtable(b);
    let b = p.declare_vtable_for_ty(trait_name, <i32>::get_type());
    let vtable2 = p.finish_vtable(b);

    let f = {
        let mut f = p.declare_function();
        let dummy = f.declare_local::<[u32; 4]>();
        f.storage_live(dummy);

        // ordering based on address
        f.assume(eq(
            wide_pointer(dummy, 0, vtable1, trait_name),
            wide_pointer(dummy, 0, vtable1, trait_name),
        ));
        f.assume(gt(
            wide_pointer(dummy, 1, vtable1, trait_name),
            wide_pointer(dummy, 0, vtable1, trait_name),
        ));
        f.assume(lt(
            wide_pointer(dummy, 0, vtable1, trait_name),
            wide_pointer(dummy, 1, vtable1, trait_name),
        ));
        f.assume(lt(
            wide_pointer(dummy, 0, vtable2, trait_name),
            wide_pointer(dummy, 1, vtable1, trait_name),
        ));
        f.assume(lt(
            wide_pointer(dummy, 0, vtable1, trait_name),
            wide_pointer(dummy, 1, vtable2, trait_name),
        ));

        // ordering based on vtable: exactly one must be true, but by non-determinism we don't know which.
        f.assume(ne(
            wide_pointer(dummy, 0, vtable1, trait_name),
            wide_pointer(dummy, 0, vtable2, trait_name),
        ));
        let hyp1 = lt(
            wide_pointer(dummy, 0, vtable1, trait_name),
            wide_pointer(dummy, 0, vtable2, trait_name),
        );
        let hyp2 = lt(
            wide_pointer(dummy, 0, vtable2, trait_name),
            wide_pointer(dummy, 0, vtable1, trait_name),
        );
        f.assume(bool_xor(hyp1, hyp2));

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}
