use crate::*;
use miniutil::DefaultTarget;
use tests::slice::ref_as_slice;

/// Tests size_of_val works with all kinds of different pointer types
#[test]
fn different_ptr_types() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();

        fn for_ptr_ty<F: Fn(Layout) -> Type>(f: &mut FunctionBuilder, reff: F) {
            let i = f.declare_local::<u32>();
            f.storage_live(i);
            f.assume(eq(size_of_val(addr_of(i, reff(<u32>::get_layout()))), const_int(4_usize)));
        }

        for_ptr_ty(&mut f, ref_ty);
        for_ptr_ty(&mut f, ref_mut_ty);
        for_ptr_ty(&mut f, raw_ptr_ty);
        for_ptr_ty(&mut f, box_ty);

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

/// Tests size_of_val for integers
#[test]
fn size_of_ints() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();

        assume_size_of_ty_conv::<u8>(&mut f, 1);
        assume_size_of_ty_conv::<i8>(&mut f, 1);
        assume_size_of_ty_conv::<u16>(&mut f, 2);
        assume_size_of_ty_conv::<i16>(&mut f, 2);
        assume_size_of_ty_conv::<u32>(&mut f, 4);
        assume_size_of_ty_conv::<i32>(&mut f, 4);
        assume_size_of_ty_conv::<u64>(&mut f, 8);
        assume_size_of_ty_conv::<i64>(&mut f, 8);

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

/// Tests size_of_val for pointers
#[test]
fn size_of_ptr() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();

        assume_size_of_ty_conv::<&u8>(&mut f, 8);
        assume_size_of_ty_conv::<&bool>(&mut f, 8);
        assume_size_of_ty_conv::<&()>(&mut f, 8);
        assume_size_of_ty_conv::<&[u8]>(&mut f, 16);
        assume_size_of_ty_conv::<&[u8; 2]>(&mut f, 8);

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

/// Tests size_of_val for zero sized types
#[test]
fn size_of_zst() {
    let mut p = ProgramBuilder::new();

    let dummy_f = {
        let mut f = p.declare_function();
        f.exit();
        p.finish_function(f)
    };

    let f = {
        let mut f = p.declare_function();

        assume_size_of_ty(&mut f, 0, <()>::get_type());
        assume_size_of_ty(&mut f, 0, <[u32; 0]>::get_type());

        // function "values" are zero sized
        let dummy_p = f.declare_local_with_ty(Type::Ptr(PtrType::FnPtr));
        f.storage_live(dummy_p);
        f.assign(dummy_p, fn_ptr(dummy_f));
        f.assume(eq(size_of_val(load(dummy_p)), const_int(0_usize)));

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

/// Tests size_of_val for tuple types
#[test]
fn size_of_struct() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        assume_size_of_ty(
            &mut f,
            16,
            tuple_ty(
                &[(size(0), <u64>::get_type()), (size(8), <u32>::get_type())],
                size(16),
                align(8),
            ),
        );
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

/// Tests size_of_val for a simple enum type
#[test]
fn size_of_enum() {
    let mut p = ProgramBuilder::new();

    // copied from `enum_representation::simple_two_variant_works`
    let simple_enum_ty = {
        let bool_var_ty = enum_variant(Type::Bool, &[]);
        let empty_var_data_ty = tuple_ty(&[], size(1), align(1)); // unit with size 1
        let u8_inttype = IntType { signed: Signedness::Unsigned, size: Size::from_bytes_const(1) };
        let empty_var_ty = enum_variant(empty_var_data_ty, &[(offset(0), (u8_inttype, 2.into()))]);
        let discriminator = discriminator_branch::<u8>(
            offset(0),
            discriminator_invalid(),
            &[
                ((0, 1), discriminator_known(0)),
                ((1, 2), discriminator_known(0)),
                ((2, 3), discriminator_known(1)),
            ],
        );
        enum_ty::<u8>(&[(0, bool_var_ty), (1, empty_var_ty)], discriminator, size(1), align(1))
    };

    let f = {
        let mut f = p.declare_function();
        assume_size_of_ty(&mut f, 1, simple_enum_ty);

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

/// Tests size_of_val for slices
#[test]
fn size_of_slice() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();

        // Make arrays, get slice pointers to them and get their size
        let arr = f.declare_local::<[u32; 3]>();
        f.storage_live(arr);
        let slice_ptr = ref_as_slice::<u32>(&mut f, arr, 3);
        f.print(size_of_val(load(slice_ptr)));

        let arr = f.declare_local::<[u32; 0]>();
        f.storage_live(arr);
        let slice_ptr = ref_as_slice::<u32>(&mut f, arr, 0);
        f.print(size_of_val(load(slice_ptr)));

        let arr = f.declare_local::<[u8; 312]>();
        f.storage_live(arr);
        let slice_ptr = ref_as_slice::<u8>(&mut f, arr, 312);
        f.print(size_of_val(load(slice_ptr)));

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_eq!(get_stdout::<BasicMem>(p).unwrap(), &["12", "0", "312"]);
}

/// Tests size_of_val only works with pointers
#[test]
fn ill_non_ptr() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        f.assume(eq(size_of_val(const_int(8_u64)), const_int(8_usize)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "UnOp::SizeOfVal invalid operand");
}

fn assume_size_of_ty(f: &mut FunctionBuilder, size: usize, ty: Type) {
    let i = f.declare_local_with_ty(ty);
    f.storage_live(i);
    f.assume(eq(size_of_val(addr_of(i, ref_ty(ty.layout::<DefaultTarget>()))), const_int(size)));
}

fn assume_size_of_ty_conv<T: TypeConv>(f: &mut FunctionBuilder, size: usize) {
    assume_size_of_ty(f, size, T::get_type());
}
