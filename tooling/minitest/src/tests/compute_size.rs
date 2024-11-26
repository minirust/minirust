use crate::*;

/// Helper which builds code to assert that compute_size with the given sized type returns the expected size.
fn assume_size_of_ty(f: &mut FunctionBuilder, size: usize, ty: Type) {
    f.assume(eq(compute_size(ty, unit()), const_int(size)));
}

/// Helper to call [`assume_size_of_ty`] when [`TypeConv`] is available.
fn assume_size_of_ty_conv<T: TypeConv>(f: &mut FunctionBuilder, size: usize) {
    assume_size_of_ty(f, size, T::get_type());
}

/// Tests compute_size for integers.
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
        // we are using a 64bit target in these tests
        assume_size_of_ty_conv::<usize>(&mut f, 8);
        assume_size_of_ty_conv::<isize>(&mut f, 8);

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

/// Tests compute_size for pointers.
#[test]
fn size_of_ptr() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();

        assume_size_of_ty_conv::<&u8>(&mut f, 8);
        assume_size_of_ty_conv::<&bool>(&mut f, 8);
        assume_size_of_ty_conv::<&()>(&mut f, 8);
        assume_size_of_ty_conv::<&[u8]>(&mut f, 16);
        assume_size_of_ty_conv::<&mut [u16]>(&mut f, 16);
        assume_size_of_ty_conv::<&[u8; 2]>(&mut f, 8);

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

/// Tests compute_size for zero sized types.
#[test]
fn size_of_zst() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();

        assume_size_of_ty(&mut f, 0, <()>::get_type());
        assume_size_of_ty(&mut f, 0, <[u32; 0]>::get_type());

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

/// Tests compute_size for tuple types.
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

/// Tests compute_size for slices.
#[test]
fn size_of_slice() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();

        // Make arrays, get slice pointers to them and get their size
        let arr = f.declare_local::<[u32; 3]>();
        f.storage_live(arr);
        let slice_ptr = construct_wide_pointer(
            addr_of(arr, <&[u32; 3]>::get_type()),
            const_int(3_usize),
            <&[u32]>::get_type(),
        );
        f.assume(eq(
            compute_size(<[u32]>::get_type(), get_metadata(slice_ptr)),
            const_int(12_usize),
        ));

        let arr = f.declare_local::<[u32; 0]>();
        f.storage_live(arr);
        let slice_ptr = construct_wide_pointer(
            addr_of(arr, <&[u32; 0]>::get_type()),
            const_int(0_usize),
            <&[u32]>::get_type(),
        );
        f.assume(eq(
            compute_size(<[u32]>::get_type(), get_metadata(slice_ptr)),
            const_int(0_usize),
        ));

        let arr = f.declare_local::<[u8; 312]>();
        f.storage_live(arr);
        let slice_ptr = construct_wide_pointer(
            addr_of(arr, <&[u8; 312]>::get_type()),
            const_int(312_usize),
            <&[u8]>::get_type(),
        );
        f.assume(eq(
            compute_size(<[u8]>::get_type(), get_metadata(slice_ptr)),
            const_int(312_usize),
        ));

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

// Ill formed tests

#[test]
fn mismatched_meta_ill_formed() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        // not `usize` as expected
        f.print(compute_size(<[u32]>::get_type(), const_int(0_i32)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(
        p,
        "UnOp::ComputeSize|ComputeAlign: invalid operand type: not metadata of type",
    );
}

#[test]
fn mismatched_meta_ill_formed_sized() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        // not `()` as expected, even though the information is not needed
        f.print(compute_size(<bool>::get_type(), const_int(0_i32)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(
        p,
        "UnOp::ComputeSize|ComputeAlign: invalid operand type: not metadata of type",
    );
}
