use crate::*;

#[test]
fn atomic_fetch_success() {
    let locals = [<u32>::get_type(); 2];

    let ptr_ty = raw_ptr_ty();

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<u32>(3)),

        atomic_fetch(FetchBinOp::Add, local(1), addr_of(local(0), ptr_ty), const_int::<u32>(1), 1)
    );
    let b1 = block!(
        print(load(local(0)), 2)
    );
    let b2 = block!(
        atomic_fetch(FetchBinOp::Sub, local(1), addr_of(local(0), ptr_ty), const_int::<u32>(2), 3)
    );
    let b3 = block!(
        print(load(local(0)), 4)
    );
    let b4 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2, b3, b4]);
    let p = program(&[f]);
    
    let output = get_stdout(p).unwrap();
    assert_eq!(output[0], "4");
    assert_eq!(output[1], "2");
}

#[test]
fn atomic_fetch_arg_count() {
    let locals = [];

    let b0 = block!(
        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::AtomicFetchAndOp(BinOpInt::Add),
            arguments: list!(),
            ret: zst_place(),
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    
    assert_ub(p, "invalid number of arguments for `Intrinsic::AtomicFetchAndOp`");
}

#[test]
fn atomic_fetch_arg_1() {
    let locals = [<u32>::get_type(); 2];

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<u32>(3)),

        atomic_fetch(FetchBinOp::Add, local(1), load(local(0)), const_int::<u32>(1), 1)
    );
    let b1 = block!(
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    
    assert_ub(p, "invalid first argument to `Intrinsic::AtomicFetchAndOp`: not a pointer");
}

#[test]
fn atomic_fetch_arg_2() {
    let locals = [<u32>::get_type(); 2];

    let ptr_ty = raw_ptr_ty();

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<u32>(3)),

        atomic_fetch(FetchBinOp::Add, local(1), addr_of(local(0), ptr_ty), const_int::<u64>(1), 1)
    );
    let b1 = block!(
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    
    assert_ub(p, "invalid second argument to `Intrinsic::AtomicFetchAndOp`: not same type as return value");
}

#[test]
fn atomic_fetch_ret_ty() {
    let locals = [<[u8; 3]>::get_type(); 2];

    let ptr_ty = raw_ptr_ty();

    let const_arr = array(&[const_int::<u8>(0); 3], <u8>::get_type());

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_arr),

        atomic_fetch(FetchBinOp::Add, local(1), addr_of(local(0), ptr_ty), const_arr, 1)
    );
    let b1 = block!(
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    
    assert_ub(p, "invalid return type for `Intrinis::AtomicFetchAndOp`: only works with integers");
}

#[test]
fn atomic_fetch_int_size() {
    let locals = [<u128>::get_type(); 2];

    let ptr_ty = raw_ptr_ty();

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<u128>(3)),

        atomic_fetch(FetchBinOp::Add, local(1), addr_of(local(0), ptr_ty), const_int::<u128>(1), 1)
    );
    let b1 = block!(
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    
    assert_ub(p, "invalid return type for `Intrinsic::AtomicFetchAndOp`: size too big");
}

#[test]
fn atomic_fetch_op() {
    let locals = [<u32>::get_type(); 2];

    let ptr_ty = raw_ptr_ty();

    let b0 = block!(
        storage_live(0),
        storage_live(1),
        assign(local(0), const_int::<u32>(3)),

        Terminator::CallIntrinsic {
            intrinsic: Intrinsic::AtomicFetchAndOp(BinOpInt::Mul),
            arguments: list!( addr_of(local(0), ptr_ty), const_int::<u32>(1) ),
            ret: local(1),
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(
        exit()
    );

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);
    
    assert_ill_formed(p);
}
