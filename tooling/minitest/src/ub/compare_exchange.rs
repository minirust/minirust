use crate::*;

#[test]
fn compare_exchange_success() {
    let locals = [ <u32>::get_ptype(); 2 ];

    let ptr_ty = raw_ptr_ty( <u32>::get_layout() );

    let addr0 = addr_of(local(0), ptr_ty);

    // Can overwrite.
    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<u32>(10)),
        compare_exchange(local(1), addr0, const_int::<u32>(10), const_int::<u32>(69), 1),
    );
    let b1 = block!(
        print(load(local(0)), 2)
    );
    let b2 = block!(
        print(load(local(1)), 3)
    );

    // Doesn't if current doesn't match.
    let b3 = block!(
        compare_exchange(local(1), addr0, const_int::<u32>(420), const_int::<u32>(421), 4)
    );
    let b4 = block!(
        print(load(local(0)), 5)
    );
    let b5 = block!(
        print(load(local(1)), 6)
    );

    let b6 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2, b3, b4, b5, b6]);
    let p = program(&[f]);

    // TODO: Check logs. This needs changes from other PR.
    // 69, 10, 69, 69
    assert_stop(p);
}

#[test]
fn compare_exchange_arg_count() {
    let locals = [ <u32>::get_ptype(); 2 ];

    let ptr_ty = raw_ptr_ty( <u32>::get_layout() );
    let addr0 = addr_of(local(0), ptr_ty);

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<u32>(10)),
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::CompareExchange,
            arguments: list!(addr0),
            ret: Some(local(1)),
            next_block: Some(BbName(Name::from_internal(1))),
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "invalid number of arguments for `Intrinsic::CompareExchange`");
}

#[test]
fn compare_exchange_arg_1_value() {
    let locals = [ <u32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        compare_exchange(local(0), const_int::<u32>(0), const_int::<u32>(0), const_int::<u32>(0), 1)
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "invalid first argument to `Intrinsic::CompareExchange`");
}

#[test]
fn compare_exchange_ret_type() {
    let locals = [ <[u8; 3]>::get_ptype(); 2 ];

    let ptr_ty = raw_ptr_ty( <[u8; 3]>::get_layout() );
    let addr0 = addr_of(local(0), ptr_ty);

    let const_arr = const_array(&[const_int::<u8>(0); 3], <u8>::get_type() );

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_arr),
        compare_exchange(local(1), addr0, const_arr, const_arr, 1)
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    assert_ub(p, "invalid return type for `Intrinis::CompareExchange`, only works with integers");
}

#[test]
fn compare_exchange_arg_1_type() {
    let locals = [ <u32>::get_ptype(); 2 ];

    let ptr_ty = raw_ptr_ty( <u32>::get_layout() );
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
    assert_ub(p, "invalid second argument to `Intrinsic::CompareExchange`, not same type");
}

#[test]
fn compare_exchange_arg_2_type() {
    let locals = [ <u32>::get_ptype(); 2 ];

    let ptr_ty = raw_ptr_ty( <u32>::get_layout() );
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
    assert_ub(p, "invalid third argument to `Intrinsic::CompareExchange`, not same type");
}

#[test]
fn compare_exchange_arg_size_max() {
    let locals = [ <u128>::get_ptype(); 2 ];

    let ptr_ty = raw_ptr_ty( <u128>::get_layout() );
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
    assert_ub(p, "invalid return type for `Intrinsic::CompareExchange`, size to big");
}
