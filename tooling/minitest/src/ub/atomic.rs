use crate::*;

#[test]
fn atomic_write_success() {
    let locals = [<u32>::get_ptype()];

    let ptr_ty = raw_ptr_ty();

    // We show that atomic write actually writes by writing 1 to local(0)

    let b0 = block!(
        storage_live(0),
        assign(local(0), const_int::<u32>(0)),

        atomic_write(addr_of(local(0), ptr_ty), const_int::<u32>(1), 1)
    );
    let b1 = block!(
        if_(eq(load(local(0)), const_int::<u32>(1)), 2, 3)
    );
    let b2 = block!(exit());
    let b3 = block!(unreachable());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2, b3]);
    let p = program(&[f]);
    assert_stop(p);
}

#[test]
fn atomic_write_arg_count() {
    let b0 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::AtomicWrite,
            arguments: list!(),
            ret: None,
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &[], &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "invalid number of arguments for `Intrinsic::AtomicWrite`")
}

#[test]
fn atomic_write_arg_type1() {
    let b0 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::AtomicWrite,
            arguments: list!(const_int::<u32>(0), const_int::<u32>(0)),
            ret: None,
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &[], &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "invalid first argument to `Intrinsic::AtomicWrite`")
}

#[test]
fn atomic_write_arg_type_pow() {
    let locals = [<[u8; 3]>::get_ptype()];

    let ptr_ty = raw_ptr_ty();
    let arr = const_array(&[
        const_int::<u8>(0),
        const_int::<u8>(1),
        const_int::<u8>(69),
    ], <u8>::get_type());

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::AtomicWrite,
            arguments: list!(addr_of(local(0), ptr_ty), arr),
            ret: None,
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "invalid second argument to `Intrinsic::AtomicWrite`, size not power of two")
}

// This test assumes that we test on a memory with `MAX_ATOMIC_SIZE <= 8 byte`.
#[test]
fn atomic_write_arg_type_size() {
    let locals = [<[u64; 2]>::get_ptype()];

    let ptr_ty = raw_ptr_ty();
    let arr = const_array(&[
        const_int::<u64>(0),
        const_int::<u64>(1),
    ], <u64>::get_type());

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::AtomicWrite,
            arguments: list!(addr_of(local(0), ptr_ty), arr),
            ret: None,
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "invalid second argument to `Intrinsic::AtomicWrite`, size too big")
}

#[test]
fn atomic_write_ret_type() {
    let locals = [<u64>::get_ptype()];

    let ptr_ty = raw_ptr_ty();

    let b0 = block!(
        storage_live(0),

        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::AtomicWrite,
            arguments: list!(addr_of(local(0), ptr_ty), const_int::<u64>(0)),
            ret: Some(local(0)),
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "invalid return type for `Intrinsic::AtomicWrite`")
}

#[test]
fn atomic_read_success() {
    let locals = [<u32>::get_ptype(); 2];

    let ptr_ty = raw_ptr_ty();

    // We show that atomic read actually reads by reading 1 from local(1).
    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(1), const_int::<u32>(1)),

        atomic_read(local(0), addr_of(local(1), ptr_ty), 1)
    );
    let b1 = block!(
        if_(eq(load(local(0)), const_int::<u32>(1)), 2, 3)
    );
    let b2 = block!(exit());
    let b3 = block!(unreachable());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2, b3]);
    let p = program(&[f]);
    assert_stop(p);
}

#[test]
fn atomic_read_arg_count() {
    let locals = [ <u32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::AtomicRead,
            arguments: list!(),
            ret: Some(local(0)),
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "invalid number of arguments for `Intrinsic::AtomicRead`")
}

#[test]
fn atomic_read_arg_type() {
    let locals = [ <u32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::AtomicRead,
            arguments: list!(const_unit()),
            ret: Some(local(0)),
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "invalid first argument to `Intrinsic::AtomicRead`")
}

#[test]
fn atomic_read_ret_type_pow() {
    let locals = [ <()>::get_ptype() ];

    let ptr_ty = raw_ptr_ty();

    let b0 = block!(
        storage_live(0),
        atomic_read(local(0), addr_of(local(0), ptr_ty), 1)
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "invalid return type for `Intrinsic::AtomicRead`, size not power of two")
}

// This test assumes that we test on a memory with `MAX_ATOMIC_SIZE <= 8 byte`.
#[test]
fn atomic_read_ret_type_size() {
    let locals = [ <[u64; 2]>::get_ptype() ];

    let ptr_ty = raw_ptr_ty();

    let b0 = block!(
        storage_live(0),
        atomic_read(local(0), addr_of(local(0), ptr_ty), 1)
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "invalid return type for `Intrinsic::AtomicRead`, size too big")
}
