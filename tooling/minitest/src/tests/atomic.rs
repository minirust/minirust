use crate::*;

#[test]
fn atomic_store_success() {
    let locals = [<u32>::get_type()];

    let ptr_ty = raw_ptr_ty();

    // We show that atomic store actually writes by writing 1 to local(0)

    let b0 = block!(
        storage_live(0),
        assign(local(0), const_int::<u32>(0)),
        atomic_store(addr_of(local(0), ptr_ty), const_int::<u32>(1), 1)
    );
    let b1 = block!(if_(eq(load(local(0)), const_int::<u32>(1)), 2, 3));
    let b2 = block!(exit());
    let b3 = block!(unreachable());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2, b3]);
    let p = program(&[f]);
    assert_stop::<BasicMem>(p);
}

#[test]
fn atomic_store_arg_count() {
    let b0 = block!(Terminator::Intrinsic {
        intrinsic: IntrinsicOp::AtomicStore,
        arguments: list!(),
        ret: zst_place(),
        next_block: Some(BbName(Name::from_internal(1)))
    });
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &[], &[b0, b1]);
    let p = program(&[f]);
    assert_ub::<BasicMem>(p, "invalid number of arguments for `AtomicStore` intrinsic")
}

#[test]
fn atomic_store_arg_type1() {
    let b0 = block!(Terminator::Intrinsic {
        intrinsic: IntrinsicOp::AtomicStore,
        arguments: list!(const_int::<u32>(0), const_int::<u32>(0)),
        ret: zst_place(),
        next_block: Some(BbName(Name::from_internal(1)))
    });
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &[], &[b0, b1]);
    let p = program(&[f]);
    assert_ub::<BasicMem>(p, "invalid first argument to `AtomicStore` intrinsic: not a pointer")
}

#[test]
fn atomic_store_arg_type_pow() {
    let locals = [<[u8; 3]>::get_type()];

    let ptr_ty = raw_ptr_ty();
    let arr =
        array(&[const_int::<u8>(0), const_int::<u8>(1), const_int::<u8>(69)], <u8>::get_type());

    let b0 = block!(
        storage_live(0),
        Terminator::Intrinsic {
            intrinsic: IntrinsicOp::AtomicStore,
            arguments: list!(addr_of(local(0), ptr_ty), arr),
            ret: zst_place(),
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub::<BasicMem>(
        p,
        "invalid second argument to `AtomicStore` intrinsic: size not power of two",
    )
}

// This test assumes that we test on a memory with `MAX_ATOMIC_SIZE <= 8 byte`.
#[test]
fn atomic_store_arg_type_size() {
    let locals = [<[u64; 2]>::get_type()];

    let ptr_ty = raw_ptr_ty();
    let arr = array(&[const_int::<u64>(0), const_int::<u64>(1)], <u64>::get_type());

    let b0 = block!(
        storage_live(0),
        Terminator::Intrinsic {
            intrinsic: IntrinsicOp::AtomicStore,
            arguments: list!(addr_of(local(0), ptr_ty), arr),
            ret: zst_place(),
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub::<BasicMem>(p, "invalid second argument to `AtomicStore` intrinsic: size too big")
}

#[test]
fn atomic_store_ret_type() {
    let locals = [<u64>::get_type()];

    let ptr_ty = raw_ptr_ty();

    let b0 = block!(
        storage_live(0),
        Terminator::Intrinsic {
            intrinsic: IntrinsicOp::AtomicStore,
            arguments: list!(addr_of(local(0), ptr_ty), const_int::<u64>(0)),
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub::<BasicMem>(p, "invalid return type for `AtomicStore` intrinsic")
}

#[test]
fn atomic_load_success() {
    let locals = [<u32>::get_type(); 2];

    let ptr_ty = raw_ptr_ty();

    // We show that atomic load actually reads by reading 1 from local(1).
    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(1), const_int::<u32>(1)),
        atomic_load(local(0), addr_of(local(1), ptr_ty), 1)
    );
    let b1 = block!(if_(eq(load(local(0)), const_int::<u32>(1)), 2, 3));
    let b2 = block!(exit());
    let b3 = block!(unreachable());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2, b3]);
    let p = program(&[f]);
    assert_stop::<BasicMem>(p);
}

#[test]
fn atomic_load_arg_count() {
    let locals = [<u32>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::Intrinsic {
            intrinsic: IntrinsicOp::AtomicLoad,
            arguments: list!(),
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub::<BasicMem>(p, "invalid number of arguments for `AtomicLoad` intrinsic")
}

#[test]
fn atomic_load_arg_type() {
    let locals = [<u32>::get_type()];

    let b0 = block!(
        storage_live(0),
        Terminator::Intrinsic {
            intrinsic: IntrinsicOp::AtomicLoad,
            arguments: list!(unit()),
            ret: local(0),
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub::<BasicMem>(p, "invalid first argument to `AtomicLoad` intrinsic: not a pointer")
}

#[test]
fn atomic_load_ret_type_pow() {
    let locals = [<()>::get_type()];

    let ptr_ty = raw_ptr_ty();

    let b0 = block!(storage_live(0), atomic_load(local(0), addr_of(local(0), ptr_ty), 1));
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub::<BasicMem>(
        p,
        "invalid return type for `AtomicLoad` intrinsic: size not power of two",
    )
}

// This test assumes that we test on a memory with `MAX_ATOMIC_SIZE <= 8 byte`.
#[test]
fn atomic_load_ret_type_size() {
    let locals = [<[u64; 2]>::get_type()];

    let ptr_ty = raw_ptr_ty();

    let b0 = block!(storage_live(0), atomic_load(local(0), addr_of(local(0), ptr_ty), 1));
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub::<BasicMem>(p, "invalid return type for `AtomicLoad` intrinsic: size too big")
}
