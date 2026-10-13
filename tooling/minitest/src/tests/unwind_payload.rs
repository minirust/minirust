use crate::*;

/// This tests the basic functionality of the panic payload.
#[test]
fn basic_test() {
    let mut p = ProgramBuilder::new();
    let f = {
        // mut x : i32 = 5;
        // x_ptr = &mut x as *mut u8
        let mut f = p.declare_function();
        let x = f.declare_local::<i32>();
        let x_ptr = f.declare_local::<*mut u8>();
        let ret = f.declare_local::<*mut u8>();
        f.storage_live(x);
        f.storage_live(x_ptr);
        f.assign(x, const_int(5));
        f.assign(x_ptr, addr_of(x, <*mut u8>::get_type()));

        // StartUnwind(x_ptr);
        // ret = get_unwind_payload();
        // StopUnwind();
        let cont = f.declare_block();
        let catch_block = f.catch_block(|f| {
            f.storage_live(ret);
            f.get_unwind_payload(ret);
            f.stop_unwind(cont);
        });
        f.start_unwind(load(x_ptr), catch_block);
        f.set_cur_block(cont, BbKind::Regular);

        // assume(x_ptr == ret)
        f.assume(eq(load(x_ptr), load(ret)));
        // assume(5 == *ret)
        f.assume(eq(const_int(5), load(deref(load(ret), <i32>::get_type()))));
        // *ret = 3;
        f.assign(deref(load(ret), <i32>::get_type()), const_int(3));
        // assume(x == 3);
        f.assume(eq(load(x), const_int(3)));
        f.exit();
        p.finish_function(f)
    };
    let p = p.finish_program(f);
    dump_program(p);
    assert_stop::<BasicMem>(p);
}

/// Tests a recursive function that panics, recurses, and catches each panic.
/// Verifies that `get_unwind_payload()` returns the correct payload for each panic.
/// ```rust
/// fn rec_fn(mut x: i32) {
///     if x == 0 {
///         return;
///     }
///     let x_ptr = &mut x as *mut u8
///     StartUnwind(x_ptr)
///     rec_fn(x-1);
///     let payload = get_unwind_payload();
///     assume(*payload == x);
///     print(*payload);
///     StopUnwind;
///     return;
/// }
///
/// fn main() {
///     rec_fn(5);
/// }
/// ```
#[test]
fn recursive_payload_test() {
    let mut p = ProgramBuilder::new();

    let rec_fn = {
        let mut f = p.declare_function();
        let x = f.declare_arg::<i32>();
        let x_ptr = f.declare_local::<*mut u8>();
        let payload = f.declare_local::<*mut u8>();
        let bb1 = f.declare_block();
        let bb2 = f.declare_block();

        f.if_(eq(load(x), const_int(0)), |f| f.return_(), |f| f.goto(bb1));
        f.set_cur_block(bb1, BbKind::Regular);

        f.storage_live(x_ptr);
        f.assign(
            x_ptr,
            ValueExpr::AddrOf {
                target: GcCow::new(x),
                ptr_ty: PtrType::Raw { meta_kind: PointerMetaKind::None },
            },
        );
        let cleanup = f.catch_block(|f| {
            f.call_nounwind(
                unit_place(),
                fn_ptr(f.name()),
                &[by_value(ValueExpr::BinOp {
                    operator: BinOp::Int(IntBinOp::Sub),
                    left: GcCow::new(load(x)),
                    right: GcCow::new(const_int(1)),
                })],
            );
            f.storage_live(payload);
            f.get_unwind_payload(payload);
            f.assume(eq(load(deref(load(payload), Type::Int(IntType::I32))), load(x)));
            f.print(load(deref(load(payload), Type::Int(IntType::I32))));
            f.stop_unwind(bb2);
        });

        f.start_unwind(load(x_ptr), cleanup);

        f.set_cur_block(bb2, BbKind::Regular);
        f.return_();
        p.finish_function(f)
    };

    let main_fn = {
        let mut f = p.declare_function();
        f.call_nounwind(unit_place(), fn_ptr(rec_fn), &[by_value(const_int(5))]);
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_eq!(get_stdout::<BasicMem>(p).unwrap(), &["1", "2", "3", "4", "5"]);
}

/// Calls `get_unwind_payload()` with an empty payload stack. Results in ub.
#[test]
fn empty_stack() {
    let mut p = ProgramBuilder::new();
    let f = {
        // mut x : i32 = 5;
        // x_ptr = &mut x as *mut u8
        let mut f = p.declare_function();
        let x = f.declare_local::<i32>();
        let x_ptr = f.declare_local::<*mut u8>();
        let payload = f.declare_local::<*mut u8>();
        f.storage_live(x);
        f.storage_live(x_ptr);
        f.assign(x, const_int(5));
        f.assign(x_ptr, addr_of(x, <*mut u8>::get_type()));

        let cont = f.declare_block();
        let catch_block = f.catch_block(|f| f.stop_unwind(cont));

        f.start_unwind(load(x_ptr), catch_block);

        f.set_cur_block(cont, BbKind::Regular);
        f.storage_live(payload);
        f.get_unwind_payload(payload);
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    dump_program(p);
    assert_ub::<BasicMem>(p, "GetUnwindPayload: the payload stack is empty");
}

/// In this test the return place of `get_unwind_payload()` has the wrong type. Results in ub.
#[test]
fn wrong_ret_ty() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let cleanup = f.cleanup_block(|f| {
            let x = f.declare_local::<i32>();
            f.storage_live(x);
            f.get_unwind_payload(x);
            f.abort();
        });
        f.start_unwind(unit_ptr(), cleanup);
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    dump_program(p);
    assert_ub::<BasicMem>(p, "invalid return type for `GetUnwindPayload` intrinsic");
}

/// In this test the unwind payload has the wrong type. Results in ub.
#[test]
fn payload_wrong_ty() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let cleanup = f.cleanup_block(|f| {
            f.abort();
        });

        let x = f.declare_local::<i32>();
        f.storage_live(x);
        f.assign(x, const_int(0));
        f.start_unwind(load(x), cleanup);
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    dump_program(p);
    assert_ill_formed::<BasicMem>(
        p,
        "Terminator::StartUnwind: the unwind payload should be a raw pointer",
    );
}
