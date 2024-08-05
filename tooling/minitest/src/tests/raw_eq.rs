use crate::*;

#[test]
fn true_raw_eq() {
    let mut p = ProgramBuilder::new();
    let mut f = p.declare_function();
    let dest = f.declare_local::<bool>();
    let left = f.declare_local::<[u8; 2]>();
    let right = f.declare_local::<[u8; 2]>();

    let pointee = layout(size(2), align(1));
    let ptr_ty = ref_ty(pointee);

    let left_ptr = addr_of(left, ptr_ty);
    let right_ptr = addr_of(right, ptr_ty);

    f.storage_live(dest);
    f.storage_live(left);
    f.storage_live(right);

    f.assign(left, array(&[const_int(42u8); 2], <u8>::get_type()));
    f.assign(right, array(&[const_int(42u8); 2], <u8>::get_type()));

    f.raw_eq(dest, left_ptr, right_ptr);

    f.assume(eq(bool_to_int::<u8>(load(dest)), bool_to_int::<u8>(const_bool(true))));
    f.exit();

    let f = p.finish_function(f);
    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn false_raw_eq() {
    let mut p = ProgramBuilder::new();
    let mut f = p.declare_function();
    let dest = f.declare_local::<bool>();
    let left = f.declare_local::<[u8; 2]>();
    let right = f.declare_local::<[u8; 2]>();

    let pointee = layout(size(2), align(1));
    let ptr_ty = ref_ty(pointee);

    let left_ptr = addr_of(left, ptr_ty);
    let right_ptr = addr_of(right, ptr_ty);

    f.storage_live(dest);
    f.storage_live(left);
    f.storage_live(right);

    f.assign(left, array(&[const_int(42u8); 2], <u8>::get_type()));
    f.assign(right, array(&[const_int(57u8); 2], <u8>::get_type()));

    f.raw_eq(dest, left_ptr, right_ptr);

    f.assume(eq(bool_to_int::<u8>(load(dest)), bool_to_int::<u8>(const_bool(false))));
    f.exit();

    let f = p.finish_function(f);
    let p = p.finish_program(f);
    assert_stop::<BasicMem>(p);
}

#[test]
fn uninit_raw_eq() {
    let mut p = ProgramBuilder::new();
    let mut f = p.declare_function();
    let dest = f.declare_local::<bool>();
    let left = f.declare_local::<[u8; 2]>();
    let right = f.declare_local::<[u8; 2]>();

    f.storage_live(dest);
    f.storage_live(left);
    f.storage_live(right);

    let pointee = layout(size(2), align(1));
    let ptr_ty = ref_ty(pointee);

    let left_ptr = addr_of(left, ptr_ty);
    let right_ptr = addr_of(right, ptr_ty);

    f.raw_eq(dest, left_ptr, right_ptr);
    f.exit();

    let f = p.finish_function(f);
    let p = p.finish_program(f);
    assert_ub::<BasicMem>(p, "invalid argument to `RawEq` intrinsic: byte is uninitialized");
}

#[test]
fn raw_ptr_raw_eq() {
    let mut p = ProgramBuilder::new();
    let mut f = p.declare_function();
    let dest = f.declare_local::<bool>();
    let left = f.declare_local::<[u8; 2]>();
    let right = f.declare_local::<[u8; 2]>();

    let ptr_ty = raw_ptr_ty();

    let left_ptr = addr_of(left, ptr_ty);
    let right_ptr = addr_of(right, ptr_ty);

    f.storage_live(dest);
    f.storage_live(left);
    f.storage_live(right);

    f.raw_eq(dest, left_ptr, right_ptr);
    f.exit();

    let f = p.finish_function(f);
    let p = p.finish_program(f);
    assert_ub::<BasicMem>(p, "invalid argument to `RawEq` intrinsic: not a reference");
}

#[test]
fn invalid_ret_ty_raw_eq() {
    let mut p = ProgramBuilder::new();
    let mut f = p.declare_function();
    let dest = f.declare_local::<i64>(); // this is the invalid return type (should be `bool`)
    let left = f.declare_local::<[u8; 2]>();
    let right = f.declare_local::<[u8; 2]>();

    let pointee = layout(size(2), align(1));
    let ptr_ty = ref_ty(pointee);

    let left_ptr = addr_of(left, ptr_ty);
    let right_ptr = addr_of(right, ptr_ty);

    f.storage_live(dest);
    f.storage_live(left);
    f.storage_live(right);

    f.raw_eq(dest, left_ptr, right_ptr);
    f.exit();

    let f = p.finish_function(f);
    let p = p.finish_program(f);
    assert_ub::<BasicMem>(p, "invalid return type for `RawEq` intrinsic");
}

#[test]
fn unequal_args_ty_raw_eq() {
    let mut p = ProgramBuilder::new();
    let mut f = p.declare_function();
    let dest = f.declare_local::<bool>();
    let left = f.declare_local::<[u8; 2]>();
    let right = f.declare_local::<[u8; 3]>(); // not the same type as `left`

    let l_pointee = layout(size(2), align(1));
    let l_ptr_ty = ref_ty(l_pointee);

    let r_pointee = layout(size(3), align(1));
    let r_ptr_ty = ref_ty(r_pointee);

    let left_ptr = addr_of(left, l_ptr_ty);
    let right_ptr = addr_of(right, r_ptr_ty);

    f.storage_live(dest);
    f.storage_live(left);
    f.storage_live(right);

    f.raw_eq(dest, left_ptr, right_ptr);
    f.exit();

    let f = p.finish_function(f);
    let p = p.finish_program(f);
    assert_ub::<BasicMem>(
        p,
        "invalid arguments to `RawEq` intrinsic: types of arguments are not identical",
    );
}

#[test]
fn invalid_arg_ty_raw_eq() {
    let mut p = ProgramBuilder::new();
    let mut f = p.declare_function();

    let dest = f.declare_local::<bool>();
    f.storage_live(dest);
    f.raw_eq(dest, const_int(8usize), const_int(8usize));
    f.exit();

    let f = p.finish_function(f);
    let p = p.finish_program(f);
    assert_ub::<BasicMem>(p, "invalid first argument to `RawEq` intrinsic: not a pointer");
}
