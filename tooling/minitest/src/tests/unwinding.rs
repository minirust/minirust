use crate::*;

#[test]
fn abort_in_cleanup() {
    let mut p = ProgramBuilder::new();
    let mut f = p.declare_function();

    let cleanup = f.cleanup(|f| {
        f.abort();
    });

    f.start_unwind(cleanup);
    let f = p.finish_function(f);
    let p = p.finish_program(f);
    dump_program(p);
    assert_abort::<BasicMem>(p, "aborted");
}

#[test]
fn start_unwind_in_main() {
    let mut p = ProgramBuilder::new();
    let mut f = p.declare_function();

    let cleanup = f.cleanup(|f| {
        f.print(const_int(42));
        f.exit(); //Call exit() instead of abort, because there is currently no way to access the standard output if the program has aborted.
    });
    f.start_unwind(cleanup);
    let f = p.finish_function(f);
    let p = p.finish_program(f);
    dump_program(p);
    assert_eq!(get_stdout::<BasicMem>(p).unwrap(), &["42"]);
}

#[test]
fn unwind_recursive_func() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let arg = f.declare_arg::<i32>();
        let var = f.declare_local::<i32>();
        let ret = f.declare_ret::<i32>();

        let cleanup_resume = f.cleanup_resume();
        let cleanup_print = f.cleanup(|f| {
            f.print(load(arg));
            f.goto(cleanup_resume);
        });
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

        let cleanup = main_fn.cleanup(|f| {
            f.exit();
        });

        main_fn.storage_live(var);
        main_fn.call(var, fn_ptr(f), &[by_value(const_int(10))], cleanup);
        main_fn.storage_dead(var);
        main_fn.exit();
        p.finish_function(main_fn)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_eq!(get_stdout::<BasicMem>(p).unwrap(), &[
        "1", "2", "3", "4", "5", "6", "7", "8", "9", "10"
    ]);
}

#[test]
fn statements_in_cleanup() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let arg = f.declare_arg::<i32>();
        let var = f.declare_local::<i32>();
        let _ret = f.declare_ret::<i32>();
        let cleanup = f.cleanup(|f| {
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

        let cleanup = main_fn.cleanup(|f| {
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

#[test]
fn print_after_unwind() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        let cleanup = f.cleanup_resume();
        f.start_unwind(cleanup);
        p.finish_function(f)
    };

    let main_fn = {
        let mut main_fn = p.declare_function();
        let cleanup = main_fn.cleanup(|f| {
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

#[test]
fn resume_in_main() {
    let mut p = ProgramBuilder::new();
    let mut f = p.declare_function();
    let cleanup = f.cleanup_resume();
    f.start_unwind(cleanup);
    let f = p.finish_function(f);
    let p = p.finish_program(f);
    dump_program(p);
    assert_ub::<BasicMem>(p, "the start function must not resume");
}

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
                let cleanup = f.cleanup_resume();
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

        let cleanup = main_fn.cleanup(|f| {
            f.abort();
        });
        main_fn.call(unit_place(), fn_ptr(f), &[by_value(const_int(3))], cleanup);
        main_fn.exit();

        p.finish_function(main_fn)
    };
    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_ub::<BasicMem>(p, "unwinding from a function where caller did not specify unwind_block");
}
