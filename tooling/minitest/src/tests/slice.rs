use crate::*;
use std::{u32, u8};

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
    dump_program(p);
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
    dump_program(p);
    assert_stop::<BasicMem>(p);
}

/// Tests that a wide pointer can be transmuted from a `(&T, usize)`.
#[test]
fn wide_pointer_transmute() {
    fn make_slice_ptr_tuple<T: TypeConv>() -> Type {
        tuple_ty(&[(size(0), <&T>::get_type()), (size(8), <u64>::get_type())], size(16), align(1))
    }

    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        // Make array
        let arr = f.declare_local::<[u32; 3]>();
        f.storage_live(arr);
        f.assign(index(arr, const_int(0)), const_int(42_u32));
        f.assign(index(arr, const_int(1)), const_int(43_u32));
        f.assign(index(arr, const_int(2)), const_int(44_u32));
        // construct fake wide ptr
        let arr_ref = addr_of(arr, ref_ty(<u32>::get_layout()));
        let fake_ptr = f.declare_local_with_ty(make_slice_ptr_tuple::<u32>());
        f.storage_live(fake_ptr);
        f.assign(field(fake_ptr, 0), arr_ref);
        f.assign(field(fake_ptr, 1), const_int(3_u64));
        f.validate(fake_ptr, false);
        // transmute into slice ref
        let slice = f.declare_local::<&[u32]>();
        f.storage_live(slice);
        f.assign(slice, transmute(load(fake_ptr), <&[u32]>::get_type()));
        f.validate(slice, false);
        // Print slice[2]
        let loaded_val = load(index(deref(load(slice), <[u32]>::get_type()), const_int(1)));
        f.print(loaded_val);
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    dump_program(p);
    assert_eq!(get_stdout::<BasicMem>(p).unwrap(), &["43"]);
}

// TODO: ill formed tests & UB
