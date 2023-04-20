use crate::*;

#[test]
fn join_arg_count() {
    let locals = [ <()>::get_ptype() ];

    let b0 = block!(
        Terminator::CallIntrinsic { 
            intrinsic: Intrinsic::Join, 
            arguments: list!(), 
            ret: None, 
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);

    assert_ub(p, "invalid number of arguments for `Intrinsic::Join`");
}

#[test]
fn join_arg_value() {
    let locals = [ <()>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        Terminator::CallIntrinsic { 
            intrinsic: Intrinsic::Join, 
            arguments: list!(load(local(0))), 
            ret: None, 
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);

    assert_ub(p, "invalid first argument to `Intrinsic::Join`");
}


#[test]
fn join_no_thread() {
    let locals = [ <u32>::get_ptype() ];

    let b0 = block!(
        storage_live(0),
        //Valid since the main thread has Id 0.
        assign(local(0), const_int::<u32>(1)),
        Terminator::CallIntrinsic { 
            intrinsic: Intrinsic::Join, 
            arguments: list!(load(local(0))), 
            ret: None, 
            next_block: Some(BbName(Name::from_internal(1)))
        }
    );
    let b1 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1]);
    let p = program(&[f]);

    assert_ub(p, "`Intrinsic::Join`: join non existing thread");
}
