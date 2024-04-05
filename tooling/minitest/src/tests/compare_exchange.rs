use crate::*;

#[test]
fn compare_exchange_success() {
    let locals = [<u32>::get_type(); 2];

    let ptr_ty = raw_ptr_ty();

    let addr0 = addr_of(local(0), ptr_ty);

    // Success case: check that we do perform a store.
    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<u32>(0)),
        compare_exchange(local(1), addr0, const_int::<u32>(0), const_int::<u32>(1), 1),
    );
    let b1 = block!(
        // print value of CASed location
        print(load(local(0)), 2)
    );
    let b2 = block!(
        // print CAS return value
        print(load(local(1)), 3)
    );

    // Failure case: check that we do not perform a store
    let b3 =
        block!(compare_exchange(local(1), addr0, const_int::<u32>(3), const_int::<u32>(42), 4));
    let b4 = block!(
        // print value of CASed location
        print(load(local(0)), 5)
    );
    let b5 = block!(
        // print CAS return value
        print(load(local(1)), 6)
    );
    let b6 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2, b3, b4, b5, b6]);
    let p = program(&[f]);

    // Check that we exchange in the first case but not the second
    let out = match get_stdout(p) {
        Ok(out) => out,
        Err(err) => panic!("{:?}", err),
    };
    assert_eq!(out, &["1", "0", "1", "1"]);
}

#[test]
fn compare_exchange_arg_count() {
    let locals = [<u32>::get_type(); 2];

    let ptr_ty = raw_ptr_ty();
    let addr0 = addr_of(local(0), ptr_ty);

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<u32>(10)),
        Terminator::Intrinsic {
            intrinsic: IntrinsicOp::AtomicCompareExchange,
            arguments: list!(addr0),
            ret: local(1),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "invalid number of arguments for `AtomicCompareExchange` intrinsic");
}

#[test]
fn compare_exchange_arg_1_value() {
    let locals = [<u32>::get_type()];

    let b0 = block!(
        storage_live(0),
        compare_exchange(
            local(0),
            const_int::<u32>(0),
            const_int::<u32>(0),
            const_int::<u32>(0),
            1
        )
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "invalid first argument to `AtomicCompareExchange` intrinsic: not a pointer");
}

#[test]
fn compare_exchange_ret_type() {
    let locals = [<[u8; 3]>::get_type(); 2];

    let ptr_ty = raw_ptr_ty();
    let addr0 = addr_of(local(0), ptr_ty);
    let const_arr = array(&[const_int::<u8>(0); 3], <u8>::get_type());

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_arr),
        compare_exchange(local(1), addr0, const_arr, const_arr, 1)
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(
        p,
        "invalid return type for `Intrinis::AtomicCompareExchange`: only works with integers",
    );
}

#[test]
fn compare_exchange_arg_1_type() {
    let locals = [<u32>::get_type(); 2];

    let ptr_ty = raw_ptr_ty();
    let addr0 = addr_of(local(0), ptr_ty);

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<u32>(0)),
        compare_exchange(local(1), addr0, const_int::<i32>(0), const_int::<u32>(0), 1)
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(
        p,
        "invalid second argument to `AtomicCompareExchange` intrinsic: not same type as return value",
    );
}

#[test]
fn compare_exchange_arg_2_type() {
    let locals = [<u32>::get_type(); 2];

    let ptr_ty = raw_ptr_ty();
    let addr0 = addr_of(local(0), ptr_ty);

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<u32>(0)),
        compare_exchange(local(1), addr0, const_int::<u32>(0), const_int::<i32>(0), 1)
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(
        p,
        "invalid third argument to `AtomicCompareExchange` intrinsic: not same type as return value",
    );
}

#[test]
fn compare_exchange_arg_size_max() {
    let locals = [<u128>::get_type(); 2];

    let ptr_ty = raw_ptr_ty();
    let addr0 = addr_of(local(0), ptr_ty);

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<u128>(0)),
        compare_exchange(local(1), addr0, const_int::<u128>(0), const_int::<u128>(0), 1)
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "invalid return type for `AtomicCompareExchange` intrinsic: size too big");
}
