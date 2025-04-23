use crate::*;

use super::bool;

/// This test creates a pointer to an `i32` local `x` in the main function.
/// The pointer is used as the data pointer for `catch_unwind`.
/// The try function writes to `x` and returns without unwinding.
/// Therefore, the write to `x` in the catch function should not be executed.
#[test]
fn no_unwind() {
    let mut p = ProgramBuilder::new();

    let try_fn = {
        let mut f = p.declare_function();
        let data_ptr = f.declare_arg::<*mut u8>();
        f.assign(deref(load(data_ptr), <i32>::get_type()), const_int(12));
        f.return_();
        p.finish_function(f)
    };

    let catch_fn = {
        let mut f = p.declare_function();
        let data_ptr = f.declare_arg::<*mut u8>();
        f.assign(deref(load(data_ptr), <i32>::get_type()), const_int(25));
        f.return_();
        p.finish_function(f)
    };

    let main_fn = {
        let mut f = p.declare_function();
        let x = f.declare_local::<i32>();
        let y = f.declare_local::<i32>();
        let data_ptr = f.declare_local::<*mut u8>();
        f.storage_live(x);
        f.storage_live(y);
        f.storage_live(data_ptr);
        f.assign(x, const_int(42));
        f.assign(data_ptr, addr_of(x, <*mut u8>::get_type()));
        f.catch_unwind(fn_ptr(try_fn), load(data_ptr), fn_ptr(catch_fn), y);

        // `x` should have the value assigned by `try_fn`
        f.assume(eq(load(x), const_int(12)));
        // The return value shold be 0, since `try_fn` does not unwind.
        f.assume(eq(load(y), const_int(0)));

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_stop::<BasicMem>(p);
}

/// This test creates a pointer to an `i32` local `x` in the main function.
/// The pointer is used as a data pointer for `catch_unwind`.
/// The try function writes to `x` and starts unwinding.
/// The catch function is executed and modifies `x`.
#[test]
fn try_fn_unwinds() {
    let mut p = ProgramBuilder::new();

    let try_fn = {
        let mut f = p.declare_function();
        let data_ptr = f.declare_arg::<*mut u8>();
        f.assign(deref(load(data_ptr), <i32>::get_type()), const_int(12));
        let cleanup = f.cleanup_block(|f| f.resume_unwind());
        f.start_unwind(cleanup);
        p.finish_function(f)
    };

    let catch_fn = {
        let mut f = p.declare_function();
        let data_ptr = f.declare_arg::<*mut u8>();
        f.assign(deref(load(data_ptr), <i32>::get_type()), const_int(25));
        f.return_();
        p.finish_function(f)
    };

    let main_fn = {
        let mut f = p.declare_function();
        let x = f.declare_local::<i32>();
        let y = f.declare_local::<i32>();
        let data_ptr = f.declare_local::<*mut u8>();
        f.storage_live(x);
        f.storage_live(y);
        f.storage_live(data_ptr);
        f.assign(x, const_int(42));
        f.assign(data_ptr, addr_of(x, <*mut u8>::get_type()));
        f.catch_unwind(fn_ptr(try_fn), load(data_ptr), fn_ptr(catch_fn), y);

        // `x` should have the value assigned by `catch_fn`.
        f.assume(eq(load(x), const_int(25)));
        // The return value shold be 1, since `try_fn` unwinds.
        f.assume(eq(load(y), const_int(1)));

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_stop::<BasicMem>(p);
}

/// Test a program where there is a second `catch_unwind` in the catch funcion.
/// A pseudo code representation of the program looks as follows:
/// ```rust
/// fn try_fn(data_ptr) {
///     z = *data_ptr;
///     z = z + 7;
///     *data_ptr = z;
///     StartUnwind;
/// }
///
/// fn inner_catch_fn(data_ptr) {
///     z = *data_ptr;
///     z = z + 19;
///     *data_ptr = z;
///     return;
/// }
///
/// fn outer_catch_fn(data_ptr) {
///     z = *data_ptr;
///     z = z + 11;
///     *data_ptr = z;
///     ret = catch_unwind(try_fn, data_ptr, inner_catch_fn);
///     assume (ret == 1);
///     return;
/// }
///
/// fn main () {
///     x = -2;
///     data_ptr = &raw x;
///     ret = catch_unwind(try_fn, data_ptr, outer_catch_fn);
///     // check if x and ret have the expected values
///     assume (x == 42);
///     assume (ret == 1);
/// }
/// ```
#[test]
fn nested_catch_unwind() {
    let mut p = ProgramBuilder::new();

    let try_fn = {
        let mut f = p.declare_function();
        let data_ptr = f.declare_arg::<*mut u8>();
        let z = f.declare_local::<i32>();
        f.storage_live(z);
        f.assign(z, load(deref(load(data_ptr), <i32>::get_type())));
        f.assign(z, add(load(z), const_int(7)));
        f.assign(deref(load(data_ptr), <i32>::get_type()), load(z));
        let cleanup = f.cleanup_block(|f| f.resume_unwind());
        f.start_unwind(cleanup);
        p.finish_function(f)
    };

    let inner_catch_fn = {
        let mut f = p.declare_function();
        let data_ptr = f.declare_arg::<*mut u8>();
        let z = f.declare_local::<i32>();
        f.storage_live(z);
        f.assign(z, load(deref(load(data_ptr), <i32>::get_type())));
        f.assign(z, add(load(z), const_int(19)));
        f.assign(deref(load(data_ptr), <i32>::get_type()), load(z));
        f.return_();
        p.finish_function(f)
    };

    let outer_catch_fn = {
        let mut f = p.declare_function();
        let data_ptr = f.declare_arg::<*mut u8>();
        let z = f.declare_local::<i32>();
        let ret = f.declare_local::<i32>();
        f.storage_live(z);
        f.storage_live(ret);
        f.assign(z, load(deref(load(data_ptr), <i32>::get_type())));
        f.assign(z, add(load(z), const_int(11)));
        f.assign(deref(load(data_ptr), <i32>::get_type()), load(z));
        f.catch_unwind(fn_ptr(try_fn), load(data_ptr), fn_ptr(inner_catch_fn), ret);

        // the return value of `catch_unwind` should be 1 since `try_fn` unwinds
        f.assume(eq(load(ret), const_int(1)));
        f.return_();
        p.finish_function(f)
    };

    let main_fn = {
        let mut f = p.declare_function();
        let x: PlaceExpr = f.declare_local::<i32>();
        let ret = f.declare_local::<i32>();
        let data_ptr = f.declare_local::<*mut u8>();
        f.storage_live(x);
        f.storage_live(ret);
        f.storage_live(data_ptr);
        f.assign(x, const_int(-2));
        f.assign(data_ptr, addr_of(x, <*mut u8>::get_type()));
        f.catch_unwind(fn_ptr(try_fn), load(data_ptr), fn_ptr(outer_catch_fn), ret);
        f.assume(eq(load(x), const_int(42)));
        f.assume(eq(load(ret), const_int(1)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_stop::<BasicMem>(p);
}

/// Test a program where there is a second `catch_unwind` in the try function.
/// A pseudo code representation of the program looks as follows:
/// ```rust
/// fn inner_try_fn(data_ptr) {
///     print(2);
///     StartUnwind;
/// }
///
/// fn inner_catch_fn(data_ptr){
///     print(3);
///     return;
/// }
///
/// fn outer_try_fn(data_ptr){
///     print(1);
///     ret = catch_unwind(inner_try_fn, data_ptr, inner_catch_fn);
///     print(4);
///     assume(ret == 1);
///     return;
/// }
///
/// // This function will not be reached.
/// fn outer_catch_fn(data_ptr){
///     print(6);
///     unreachable();
/// }
///
/// fn main_fn() {
///     ret = -1;
///     data_ptr = &raw ret;
///     print(0);
///     ret = catch_unwind(outer_try_fn, data_ptr, outer_catch_fn);
///     print(5);
///     assume(ret == 0);
/// }
/// ```
/// This program is expected to print the numbers 0 to 5 in order.
#[test]
fn catch_in_try_fn() {
    let mut p = ProgramBuilder::new();

    let inner_try_fn = {
        let mut f = p.declare_function();
        f.declare_arg::<*mut u8>();
        f.print(const_int(2));
        let cleanup = f.cleanup_block(|f| f.resume_unwind());
        f.start_unwind(cleanup);
        p.finish_function(f)
    };

    let inner_catch_fn = {
        let mut f = p.declare_function();
        f.declare_arg::<*mut u8>();
        f.print(const_int(3));
        f.return_();
        p.finish_function(f)
    };

    let outer_try_fn = {
        let mut f = p.declare_function();
        let data_ptr = f.declare_arg::<*mut u8>();
        let ret = f.declare_local::<i32>();
        f.storage_live(ret);
        f.print(const_int(1));
        f.catch_unwind(fn_ptr(inner_try_fn), load(data_ptr), fn_ptr(inner_catch_fn), ret);
        f.print(const_int(4));
        f.assume(eq(load(ret), const_int(1)));
        f.return_();
        p.finish_function(f)
    };

    let outer_catch_fn = {
        let mut f = p.declare_function();
        f.declare_arg::<*mut u8>();
        f.print(const_int(6));
        f.unreachable();
        p.finish_function(f)
    };

    let main_fn = {
        let mut f = p.declare_function();
        let ret = f.declare_local::<i32>();
        let data_ptr = f.declare_local::<*mut u8>();
        f.storage_live(ret);
        f.storage_live(data_ptr);
        f.assign(ret, const_int(-1));
        f.assign(data_ptr, addr_of(ret, <*mut u8>::get_type()));
        f.print(const_int(0));
        f.catch_unwind(fn_ptr(outer_try_fn), load(data_ptr), fn_ptr(outer_catch_fn), ret);
        f.print(const_int(5));
        f.assume(eq(load(ret), const_int(0)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_eq!(get_stdout::<BasicMem>(p).unwrap(), &["0", "1", "2", "3", "4", "5"]);
}

/// In this test, a value that is not a pointer, is used as data pointer in catch_unwind, which results in an ill-formed program.
#[test]
fn wrong_data_ptr() {
    let mut p = ProgramBuilder::new();

    let try_fn = {
        let mut f = p.declare_function();
        let _ = f.declare_arg::<*mut u8>();
        f.return_();
        p.finish_function(f)
    };

    let catch_fn = {
        let mut f = p.declare_function();
        let _ = f.declare_arg::<*mut u8>();
        f.return_();
        p.finish_function(f)
    };

    let main_fn = {
        let mut f = p.declare_function();
        let x = f.declare_local::<i32>();
        f.storage_live(x);
        f.catch_unwind(fn_ptr(try_fn), load(x), fn_ptr(catch_fn), x);
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_ill_formed::<BasicMem>(p, "Terminator::CatchUnwind: data_ptr must be a pointer");
}

/// Use an integer constant instead of a function pointer for `try_fn` in `catch_unwind`, which results in an ill-formed program.
#[test]
fn invalid_try_fn_ptr() {
    let mut p = ProgramBuilder::new();

    let catch_fn = {
        let mut f = p.declare_function();
        let _ = f.declare_arg::<*mut u8>();
        f.return_();
        p.finish_function(f)
    };

    let main_fn = {
        let mut f = p.declare_function();
        let x = f.declare_local::<i32>();
        let y = f.declare_local::<*mut u8>();
        f.storage_live(x);
        f.storage_live(y);
        f.catch_unwind(const_int(42), load(y), fn_ptr(catch_fn), x);
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_ill_formed::<BasicMem>(p, "Terminator::CatchUnwind: invalid type");
}

/// Use an integer constant instead of a function pointer for `catch_fn` in `catch_unwind`, which results in an ill-formed program.
#[test]
fn invalid_catch_fn_ptr() {
    let mut p = ProgramBuilder::new();

    let try_fn = {
        let mut f = p.declare_function();
        let _ = f.declare_arg::<*mut u8>();
        f.return_();
        p.finish_function(f)
    };

    let main_fn = {
        let mut f = p.declare_function();
        let x = f.declare_local::<i32>();
        let y = f.declare_local::<*mut u8>();
        f.storage_live(x);
        f.storage_live(y);
        f.catch_unwind(fn_ptr(try_fn), load(y), const_int(42), x);
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_ill_formed::<BasicMem>(p, "Terminator::CatchUnwind: invalid type");
}

/// In this test, the local used to save the return value of `CatchUnwind` has the wrong type which results in an ill-formed program.
#[test]
fn wrong_return_type() {
    let mut p = ProgramBuilder::new();

    let try_fn = {
        let mut f = p.declare_function();
        let _ = f.declare_arg::<*mut u8>();
        f.return_();
        p.finish_function(f)
    };

    let catch_fn = {
        let mut f = p.declare_function();
        let _ = f.declare_arg::<*mut u8>();
        f.return_();
        p.finish_function(f)
    };

    let main_fn = {
        let mut f = p.declare_function();
        let data_ptr = f.declare_local::<*mut u8>();
        let ret = f.declare_local::<bool>();
        f.storage_live(data_ptr);
        f.storage_live(ret);
        f.catch_unwind(fn_ptr(try_fn), load(data_ptr), fn_ptr(catch_fn), ret);
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_ill_formed::<BasicMem>(p, "Terminator::CatchUnwind: return type should be i32");
}

/// In this test, the catch function unwinds, which is UB.
#[test]
fn unwind_in_catch() {
    let mut p = ProgramBuilder::new();

    let try_fn = {
        let mut f = p.declare_function();
        let _ = f.declare_arg::<*mut u8>();
        let cleanup = f.cleanup_block(|f| f.resume_unwind());
        f.start_unwind(cleanup);
        p.finish_function(f)
    };

    let catch_fn = {
        let mut f = p.declare_function();
        let _ = f.declare_arg::<*mut u8>();
        let cleanup = f.cleanup_block(|f| f.resume_unwind());
        f.start_unwind(cleanup);
        p.finish_function(f)
    };

    let main_fn = {
        let mut f = p.declare_function();
        let ret = f.declare_local::<i32>();
        let data_ptr = f.declare_local::<*mut u8>();
        f.storage_live(ret);
        f.storage_live(data_ptr);
        f.assign(data_ptr, addr_of(ret, <*mut u8>::get_type()));
        f.catch_unwind(fn_ptr(try_fn), load(data_ptr), fn_ptr(catch_fn), ret);
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_ub::<BasicMem>(
        p,
        "unwinding from a function where the caller did not specify an unwind_block",
    );
}
