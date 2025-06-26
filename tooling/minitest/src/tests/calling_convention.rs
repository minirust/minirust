use crate::*;

/// In this test, the caller uses the C calling convention and the callee uses the Rust calling convention,
/// which results in ub.
#[test]
fn call_c_to_rust() {
    let mut p = ProgramBuilder::new();
    let other_fn = {
        let mut f = p.declare_function();
        f.return_();
        p.finish_function(f)
    };
    let start_fn = {
        let mut f = p.declare_function();
        let cleanup = f.cleanup_block(|f| f.abort());
        f.call_with_conv(unit_place(), fn_ptr(other_fn), &[], CallingConvention::C, cleanup);
        f.exit();
        p.finish_function(f)
    };
    let p = p.finish_program(start_fn);
    dump_program(p);
    assert_ub::<BasicMem>(p, "call ABI violation: calling conventions are not the same");
}

/// In this test, the caller uses the Rust calling convention and the callee uses the C calling convention,
/// which results in ub.
#[test]
fn call_rust_to_c() {
    let mut p = ProgramBuilder::new();
    let other_fn = {
        let mut f = p.declare_function();
        f.set_conv(CallingConvention::C);
        f.return_();
        p.finish_function(f)
    };
    let start_fn = {
        let mut f = p.declare_function();
        let cleanup = f.cleanup_block(|f| f.abort());
        f.call(unit_place(), fn_ptr(other_fn), &[], cleanup);
        f.exit();
        p.finish_function(f)
    };
    let p = p.finish_program(start_fn);
    dump_program(p);
    assert_ub::<BasicMem>(p, "call ABI violation: calling conventions are not the same");
}

/// The function provided to spawn must use the C calling convention.
#[test]
fn spawn_wrong_conv() {
    let mut p = ProgramBuilder::new();
    let other_fn = {
        let mut f = p.declare_function();
        let _ = f.declare_arg::<*mut u8>();
        f.return_();
        p.finish_function(f)
    };
    let start_fn = {
        let mut f = p.declare_function();
        let x = f.declare_local::<i32>();
        let data_ptr = f.declare_local::<*mut u8>();
        f.storage_live(x);
        f.storage_live(data_ptr);
        f.assign(data_ptr, addr_of(x, <*mut u8>::get_type()));
        f.spawn(other_fn, load(data_ptr), x);
        f.exit();
        p.finish_function(f)
    };
    let p = p.finish_program(start_fn);
    dump_program(p);
    assert_ub::<BasicMem>(p, "call ABI violation: calling conventions are not the same");
}
