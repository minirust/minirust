use crate::*;
use miniutil::DefaultTarget;

/// Helper which builds code to assert that size_of_val with given type gives the expected size.
fn assume_size_of_ty(f: &mut FunctionBuilder, size: usize, ty: Type) {
    // This is now kind of ugly, but there is no way to get a minirust reference type for a given minirust type anymore.
    let pointee = PointeeInfo {
        size: ty.size::<DefaultTarget>(),
        align: ty.align::<DefaultTarget>(),
        inhabited: true,
        freeze: false,
        unpin: false,
    };
    let i = f.declare_local_with_ty(ty);
    f.storage_live(i);
    f.assume(eq(size_of_val(addr_of(i, ref_ty(pointee))), const_int(size)));
}

/// Helper to call [`assume_size_of_ty`] when [`TypeConv`] is available.
fn assume_size_of_ty_conv<T: TypeConv>(f: &mut FunctionBuilder, size: usize) {
    assume_size_of_ty(f, size, T::get_type());
}

/// Tests size_of_val works with different kinds of pointer types.
#[test]
fn different_ptr_types() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();

        fn for_ptr_ty(f: &mut FunctionBuilder, ptr_ty_u32: Type) {
            let i = f.declare_local::<u32>();
            f.storage_live(i);
            f.assume(eq(size_of_val(addr_of(i, ptr_ty_u32)), const_int(4_usize)));
        }

        let Type::Ptr(PtrType::Ref { pointee, .. }) = <&u32>::get_type() else { panic!() };
        for_ptr_ty(&mut f, ref_ty(pointee));
        for_ptr_ty(&mut f, ref_mut_ty(pointee));

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

/// Tests size_of_val for integers.
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

/// Tests size_of_val for pointers.
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

/// Tests size_of_val for zero sized types.
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

/// Tests size_of_val for tuple types.
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

/// Tests size_of_val for slices.
#[test]
fn size_of_slice() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();

        // Make arrays, get slice pointers to them and get their size
        let arr = f.declare_local::<[u32; 3]>();
        f.storage_live(arr);
        let slice_ptr = construct_wide_pointer(
            addr_of(arr, <&()>::get_type()),
            const_int(3_usize),
            <&[u32]>::get_type(),
        );
        f.assume(eq(size_of_val(slice_ptr), const_int(12_usize)));

        let arr = f.declare_local::<[u32; 0]>();
        f.storage_live(arr);
        let slice_ptr = construct_wide_pointer(
            addr_of(arr, <&()>::get_type()),
            const_int(0_usize),
            <&[u32]>::get_type(),
        );
        f.assume(eq(size_of_val(slice_ptr), const_int(0_usize)));

        let arr = f.declare_local::<[u8; 312]>();
        f.storage_live(arr);
        let slice_ptr = construct_wide_pointer(
            addr_of(arr, <&()>::get_type()),
            const_int(312_usize),
            <&[u8]>::get_type(),
        );
        f.assume(eq(size_of_val(slice_ptr), const_int(312_usize)));

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

// Ill formed tests

/// Tests size_of_val only works with pointers.
#[test]
fn ill_non_ptr() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        f.assume(eq(size_of_val(const_int(0_u64)), const_int(8_usize)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "UnOp::SizeOfVal: invalid operand: not a reference");
}

/// Raw pointers do not have enough information to compute the size.
#[test]
fn ill_raw_ptr() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let x = f.declare_local::<u32>();
        f.assume(eq(size_of_val(addr_of(x, <*const u32>::get_type())), const_int(4_usize)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "UnOp::SizeOfVal: invalid operand: not a reference");
}

/// Box pointers are not supported.
#[test]
fn ill_box_ptr() {
    let mut p = ProgramBuilder::new();
    let Type::Ptr(PtrType::Ref { pointee, .. }) = <&u32>::get_type() else { panic!() };

    let f = {
        let mut f = p.declare_function();
        let x = f.declare_local::<u32>();
        f.assume(eq(size_of_val(addr_of(x, box_ty(pointee))), const_int(4_usize)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "UnOp::SizeOfVal: invalid operand: not a reference");
}

/// size_of_val for function pointers makes little sense, and is hence rejected.
#[test]
fn ill_fn_ptr() {
    let mut p = ProgramBuilder::new();

    let dummy_f = {
        let mut f = p.declare_function();
        f.exit();
        p.finish_function(f)
    };

    let f = {
        let mut f = p.declare_function();

        // function "values" are zero sized
        let dummy_p = f.declare_local_with_ty(Type::Ptr(PtrType::FnPtr));
        f.storage_live(dummy_p);
        f.assign(dummy_p, fn_ptr(dummy_f));
        f.assume(eq(size_of_val(load(dummy_p)), const_int(0_usize)));

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "UnOp::SizeOfVal: invalid operand: not a reference");
}
