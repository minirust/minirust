use crate::*;

/// Test if the execution reaches the terminate block. The test calls a function that panics, and then calls it again in the cleanup block.
#[test]
fn reach_terminate_block() {
    let mut p = ProgramBuilder::new();

    let panic_fn = {
        let mut f = p.declare_function();
        let resume = f.cleanup_block(|f| f.resume_unwind());
        f.start_unwind(resume);
        p.finish_function(f)
    };

    let main_fn = {
        let mut f = p.declare_function();
        let terminate = f.terminating_block(|f| {
            f.exit();
        });
        let cleanup = f.cleanup_block(|f| {
            f.call(unit_place(), fn_ptr(panic_fn), &[], terminate);
            f.unreachable();
        });

        f.call(unit_place(), fn_ptr(panic_fn), &[], cleanup);
        f.unreachable();
        p.finish_function(f)
    };
    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_stop::<BasicMem>(p);
}

/// Test a program that starts unwinding and aborts in the cleanup block.
#[test]
fn abort_in_cleanup() {
    let mut p = ProgramBuilder::new();
    let mut f = p.declare_function();

    let cleanup = f.cleanup_block(|f| {
        f.abort();
    });

    f.start_unwind(cleanup);
    let f = p.finish_function(f);
    let p = p.finish_program(f);
    dump_program(p);
    assert_abort::<BasicMem>(p);
}

/// This test calls `print` in the cleanup block and checks whether the value is actually printed.
#[test]
fn start_unwind_in_main() {
    let mut p = ProgramBuilder::new();
    let mut f = p.declare_function();

    let cleanup = f.cleanup_block(|f| {
        f.print(const_int(42));
        // Call `exit()` instead of `abort()`, because there is currently no way to access the standard output if the program has aborted.
        f.exit();
    });
    f.start_unwind(cleanup);
    let f = p.finish_function(f);
    let p = p.finish_program(f);
    dump_program(p);
    assert_eq!(get_stdout::<BasicMem>(p).unwrap(), &["42"]);
}

/// This test calls a recursive function that will panic after some recursive calls.
/// The function is structured as follows:
/// ```
/// fn recursive_fn(arg: i32){  
///     print(arg);  
///     if arg == 0 {  
///         StartUnwind();  
///     }  
///     else{  
///         recursive_fn(arg-1);  
///     }  
///     
///     --Cleanup-- {  
///         print(arg);  
///     }  
/// }
/// ```
#[test]
fn unwind_recursive_func() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let arg = f.declare_arg::<i32>();
        let var = f.declare_local::<i32>();
        let ret = f.declare_ret::<i32>();

        let cleanup_resume = f.cleanup_block(|f| f.resume_unwind());
        let cleanup_print = f.cleanup_block(|f| {
            f.print(load(arg));
            f.goto(cleanup_resume);
        });
        f.print(load(arg));
        f.if_(
            eq(load(arg), const_int(0)),
            |f| f.start_unwind(cleanup_resume),
            |f| {
                f.storage_live(var);
                f.assign(var, sub_unchecked(load(arg), const_int(1)));
                f.call(ret, fn_ptr(f.name()), &[in_place(var)], cleanup_print);
                f.storage_dead(var);
                f.return_();
            },
        );
        p.finish_function(f)
    };

    let main_fn = {
        let mut main_fn = p.declare_function();
        let var = main_fn.declare_local::<i32>();

        let cleanup = main_fn.cleanup_block(|f| {
            f.exit();
        });

        main_fn.storage_live(var);
        main_fn.call(var, fn_ptr(f), &[by_value(const_int(6))], cleanup);
        main_fn.storage_dead(var);
        main_fn.exit();
        p.finish_function(main_fn)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_eq!(get_stdout::<BasicMem>(p).unwrap(), &[
        "6", "5", "4", "3", "2", "1", "0", "1", "2", "3", "4", "5", "6",
    ]);
}

/// A test case with non-terminator statements in the cleanup block.
#[test]
fn statements_in_cleanup() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let arg = f.declare_arg::<i32>();
        let var = f.declare_local::<i32>();
        let _ret = f.declare_ret::<i32>();
        let cleanup = f.cleanup_block(|f| {
            f.assign(var, add_unchecked(load(arg), const_int(3)));
            f.assign(var, mul_unchecked(load(var), load(arg)));
            f.assign(var, shl_unchecked(load(var), const_int(2)));
            f.print(load(var));
            f.resume_unwind();
        });
        f.storage_live(var);
        f.start_unwind(cleanup);
        p.finish_function(f)
    };

    let main_fn = {
        let mut main_fn = p.declare_function();
        let var = main_fn.declare_local::<i32>();

        let cleanup = main_fn.cleanup_block(|f| {
            f.exit();
        });

        main_fn.storage_live(var);
        main_fn.call(var, fn_ptr(f), &[by_value(const_int(10))], cleanup);
        main_fn.exit();

        p.finish_function(main_fn)
    };
    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_eq!(get_stdout::<BasicMem>(p).unwrap(), &["520"]);
}

/// This test prints before and after a function call.
/// The called function panics, so only the first print statement should be executed.
#[test]
fn print_after_unwind() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let cleanup = f.cleanup_block(|f| f.resume_unwind());
        f.start_unwind(cleanup);
        p.finish_function(f)
    };

    let main_fn = {
        let mut main_fn = p.declare_function();
        let cleanup = main_fn.cleanup_block(|f| {
            f.exit();
        });

        main_fn.print(const_int(1));
        main_fn.call(unit_place(), fn_ptr(f), &[], cleanup);
        main_fn.print(const_int(2));
        main_fn.exit();
        p.finish_function(main_fn)
    };
    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_eq!(get_stdout::<BasicMem>(p).unwrap(), &["1"]);
}

/// This test resumes unwinding in the start function, which should result in UB.
#[test]
fn resume_in_main() {
    let mut p = ProgramBuilder::new();
    let mut f = p.declare_function();
    let cleanup = f.cleanup_block(|f| f.resume_unwind());
    f.start_unwind(cleanup);
    let f = p.finish_function(f);
    let p = p.finish_program(f);
    dump_program(p);
    assert_ub::<BasicMem>(p, "the function at the bottom of the stack must not unwind");
}

/// This test creates a new thread and resumes unwinding at the bottom of the stack, which should result in UB.
#[test]
fn resume_in_thread() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let _ = f.declare_arg::<*const ()>();
        let cleanup = f.cleanup_block(|f| f.resume_unwind());
        f.start_unwind(cleanup);
        p.finish_function(f)
    };

    let main_fn = {
        let mut main_fn = p.declare_function();
        let x = main_fn.declare_local::<i32>();
        main_fn.storage_live(x);
        main_fn.spawn(f, null(), x);
        main_fn.join(load(x));
        main_fn.exit();
        p.finish_function(main_fn)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_ub::<BasicMem>(p, "the function at the bottom of the stack must not unwind");
}

/// This test resumes unwinding, but no `unwind_block` is specified, which should result in UB.
#[test]
fn resume_no_unwind_block() {
    let mut p = ProgramBuilder::new();
    let f = {
        let mut f = p.declare_function();
        let arg = f.declare_arg::<i32>();
        let var = f.declare_local::<i32>();

        f.storage_live(var);
        f.assign(var, sub_unchecked(load(arg), const_int(1)));
        f.if_(
            eq(load(var), const_int(0)),
            |f| {
                let cleanup = f.cleanup_block(|f| f.resume_unwind());
                f.start_unwind(cleanup);
            },
            |f| {
                f.call_nounwind(unit_place(), fn_ptr(f.name()), &[in_place(var)]);
            },
        );
        f.return_();
        p.finish_function(f)
    };

    let main_fn = {
        let mut main_fn = p.declare_function();

        let cleanup = main_fn.cleanup_block(|f| {
            f.abort();
        });
        main_fn.call(unit_place(), fn_ptr(f), &[by_value(const_int(3))], cleanup);
        main_fn.exit();

        p.finish_function(main_fn)
    };
    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_ub::<BasicMem>(
        p,
        "unwinding from a function where the caller did not specify an unwind_block",
    );
}
