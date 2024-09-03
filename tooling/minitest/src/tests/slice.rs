use crate::*;

/// This helper is a workaround for the missing unsizing coercion.
///
/// It builds code to create a `&[T]` place from an `[T; known_len]` place.
fn ref_as_slice<T: TypeConv>(f: &mut FunctionBuilder, arr: PlaceExpr, known_len: u64) -> PlaceExpr {
    // construct fake wide ptr
    let arr_ref = addr_of(arr, <*const T>::get_type());
    let pair_ty = PtrType::Raw { meta_kind: PointerMetaKind::ElementCount }
        .as_wide_pair::<miniutil::DefaultTarget>()
        .expect("PtrType is wide");
    let fake_ptr = f.declare_local_with_ty(pair_ty);
    f.storage_live(fake_ptr);
    f.assign(field(fake_ptr, 0), arr_ref);
    f.assign(field(fake_ptr, 1), const_int(known_len));
    f.validate(fake_ptr, false); // Bad for ZST ?
    // transmute into slice ref
    let slice = f.declare_local::<&[T]>();
    f.storage_live(slice);
    f.assign(slice, transmute(load(fake_ptr), <&[T]>::get_type()));
    f.validate(slice, false);
    slice
}

/// Tests that slices can occur behind different pointer types
#[test]
fn slice_ref_wf() {
    let mut p = ProgramBuilder::new();

    let _f = {
        let mut f = p.declare_function();
        let _var = f.declare_local::<&[u32]>();
        let _ret = f.declare_ret::<&mut [u8]>();
        let _arg = f.declare_arg::<*const [[[u8; 3]; 2]]>();
        f.exit();
        p.finish_function(f)
    };

    let main = {
        let mut main = p.declare_function();
        main.exit();
        p.finish_function(main)
    };

    let p = p.finish_program(main);
    assert_stop::<BasicMem>(p);
}

/// Tests that an index operation is well formed
#[test]
fn index_wf() {
    let mut p = ProgramBuilder::new();

    let _f = {
        let mut f = p.declare_function();
        let slice = f.declare_arg::<&[u32]>();
        let var = f.declare_local::<u32>();
        f.storage_live(var);
        let elem_place = index(deref(load(slice), <[u32]>::get_type()), const_int(2));
        f.assign(elem_place, const_int(42_u32));
        f.assign(var, load(elem_place));
        f.exit();
        p.finish_function(f)
    };

    let main = {
        let mut main = p.declare_function();
        main.exit();
        p.finish_function(main)
    };

    let p = p.finish_program(main);
    assert_stop::<BasicMem>(p);
}

/// Asserts that the slice element type must be sized
#[test]
fn slice_ref_unsized_elem_not_wf() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let var = f.declare_local_with_ty(<&[u32]>::get_type());
        f.storage_live(var);
        let slice_place = deref(load(var), slice_ty(slice_ty(<u32>::get_type())));
        f.validate(slice_place, false);
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "Type::Slice: unsized element type");
}

#[test]
fn local_not_wf() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        // ill formed:
        f.declare_local_with_ty(slice_ty(<u32>::get_type()));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "Function: unsized local variable");
}

#[test]
fn load_not_wf() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let var = f.declare_local_with_ty(<&[u32]>::get_type());
        f.storage_live(var);
        let slice_place = deref(load(var), <[u32]>::get_type());
        // ill formed load: (also ill formed print, but need some way to use the valueexpr)
        f.print(load(slice_place));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "ValueExpr::Load: unsized value type");
}

#[test]
fn transmute_not_wf() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let arr = f.declare_local::<[u32; 1]>();
        f.storage_live(arr);
        f.assign(index(arr, const_int(0)), const_int(42_u32));
        // ill formed transmute:
        let slice = transmute(load(arr), <[u32]>::get_type());
        // (also ill formed print, but need some way to use the valueexpr)
        f.print(slice);
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "Cast::Transmute: unsized target type");
}

/// Tests that a wide pointer can be transmuted from a `(*T, usize)`.
#[test]
fn index_with_transmuted() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        // Make array
        let arr = f.declare_local::<[u32; 3]>();
        f.storage_live(arr);
        f.assign(index(arr, const_int(0)), const_int(42_u32));
        f.assign(index(arr, const_int(1)), const_int(43_u32));
        f.assign(index(arr, const_int(2)), const_int(44_u32));
        let slice_ptr = ref_as_slice::<u32>(&mut f, arr, 3);
        // Print slice[1]
        let loaded_val = load(index(deref(load(slice_ptr), <[u32]>::get_type()), const_int(1)));
        f.print(loaded_val);
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_eq!(get_stdout::<BasicMem>(p).unwrap(), &["43"]);
}

/// Tests that indexing into a slice throws UB for invalid indices
#[test]
fn invalid_index_ub() {
    fn for_index(idx: isize) {
        let mut p = ProgramBuilder::new();
        let f = {
            let mut f = p.declare_function();
            // Make array
            let arr = f.declare_local::<[u32; 2]>();
            f.storage_live(arr);
            f.assign(index(arr, const_int(0)), const_int(42_u32));
            f.assign(index(arr, const_int(1)), const_int(43_u32));
            let slice_ptr = ref_as_slice::<u32>(&mut f, arr, 2);
            // This should UB
            let loaded_val =
                load(index(deref(load(slice_ptr), <[u32]>::get_type()), const_int(idx)));
            f.print(loaded_val);
            f.exit();
            p.finish_function(f)
        };
        let p = p.finish_program(f);
        assert_ub::<BasicMem>(p, "access to out-of-bounds index");
    }

    for_index(-1);
    for_index(2);
}

#[test]
fn get_metadata_correct() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        // Make array
        let arr = f.declare_local::<[u32; 3]>();
        f.storage_live(arr);
        f.assign(index(arr, const_int(0)), const_int(42_u32));
        f.assign(index(arr, const_int(1)), const_int(43_u32));
        f.assign(index(arr, const_int(2)), const_int(44_u32));
        // This is now a place with type `&[u32]`
        let slice_ptr = ref_as_slice::<u32>(&mut f, arr, 3);
        // Get the metadata && assert it to be correct
        let loaded_len = get_metadata(load(slice_ptr));
        f.assume(eq(loaded_len, const_int(3_usize)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}
