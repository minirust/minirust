use crate::*;

/// Helper which builds code to assert that compute_align with the given sized type returns the expected align.
fn assume_align_of_ty(f: &mut FunctionBuilder, align: usize, ty: Type) {
    f.assume(eq(compute_align(ty, unit()), const_int(align)));
}

/// Helper to call [`assume_align_of_ty`] when [`TypeConv`] is available.
fn assume_align_of_ty_conv<T: TypeConv>(f: &mut FunctionBuilder, align: usize) {
    assume_align_of_ty(f, align, T::get_type());
}

/// Tests compute_align for integers.
#[test]
fn align_of_ints() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();

        assume_align_of_ty_conv::<u8>(&mut f, 1);
        assume_align_of_ty_conv::<i8>(&mut f, 1);
        assume_align_of_ty_conv::<u16>(&mut f, 2);
        assume_align_of_ty_conv::<i16>(&mut f, 2);
        assume_align_of_ty_conv::<u32>(&mut f, 4);
        assume_align_of_ty_conv::<i32>(&mut f, 4);
        assume_align_of_ty_conv::<u64>(&mut f, 8);
        assume_align_of_ty_conv::<i64>(&mut f, 8);
        // we are using a 64bit target in these tests
        assume_align_of_ty_conv::<usize>(&mut f, 8);
        assume_align_of_ty_conv::<isize>(&mut f, 8);

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

/// Tests compute_align for pointers.
#[test]
fn align_of_ptr() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();

        assume_align_of_ty_conv::<&u8>(&mut f, 8);
        assume_align_of_ty_conv::<&bool>(&mut f, 8);
        assume_align_of_ty_conv::<&()>(&mut f, 8);
        assume_align_of_ty_conv::<&[u8]>(&mut f, 8);
        assume_align_of_ty_conv::<&mut [u16]>(&mut f, 8);
        assume_align_of_ty_conv::<&[u8; 2]>(&mut f, 8);

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

/// Tests compute_align for zero sized types.
#[test]
fn align_of_zst() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();

        assume_align_of_ty(&mut f, 1, <()>::get_type());
        assume_align_of_ty(&mut f, 4, <[u32; 0]>::get_type());

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

/// Tests compute_align for tuple types.
#[test]
fn align_of_struct() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        assume_align_of_ty(
            &mut f,
            8,
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

/// Tests compute_align for slices.
#[test]
fn align_of_slice() {
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
            compute_align(<[u32]>::get_type(), get_metadata(slice_ptr)),
            const_int(4_usize),
        ));

        let arr = f.declare_local::<[u32; 0]>();
        f.storage_live(arr);
        let slice_ptr = construct_wide_pointer(
            addr_of(arr, <&[u32; 0]>::get_type()),
            const_int(0_usize),
            <&[u32]>::get_type(),
        );
        f.assume(eq(
            compute_align(<[u32]>::get_type(), get_metadata(slice_ptr)),
            const_int(4_usize),
        ));

        let arr = f.declare_local::<[u8; 312]>();
        f.storage_live(arr);
        let slice_ptr = construct_wide_pointer(
            addr_of(arr, <&[u8; 312]>::get_type()),
            const_int(312_usize),
            <&[u8]>::get_type(),
        );
        f.assume(eq(
            compute_align(<[u8]>::get_type(), get_metadata(slice_ptr)),
            const_int(1_usize),
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
        f.print(compute_align(<[u32]>::get_type(), const_int(0_i32)));
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
        f.print(compute_align(<bool>::get_type(), const_int(0_i32)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(
        p,
        "UnOp::ComputeSize|ComputeAlign: invalid operand type: not metadata of type",
    );
}
