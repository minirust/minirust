use crate::*;

/// In this test, `try_fn` unwinds. The panic is caught and `catch_fn` is executed.
#[test]
fn catch_unwind() {
    let mut p = ProgramBuilder::new();

    let try_fn = {
        let mut f = p.declare_function();
        f.print(const_int(2));
        let cleanup = f.cleanup_block(|f| f.resume_unwind());
        f.start_unwind(cleanup);
        p.finish_function(f)
    };

    let catch_fn = {
        let mut f = p.declare_function();
        f.print(const_int(3));
        f.return_();
        p.finish_function(f)
    };

    let main_fn = {
        let mut f = p.declare_function();
        f.print(const_int(1));

        let cont = f.declare_block();

        let catch_block = f.catch_block(|f| {
            f.call_nounwind(unit_place(), fn_ptr(catch_fn), &[]);
            f.print(const_int(5));
            f.stop_unwind(cont);
        });
        f.call(unit_place(), fn_ptr(try_fn), &[], catch_block);
        f.print(const_int(4));
        f.goto(cont);
        f.set_cur_block(cont, BbKind::Regular);
        f.print(const_int(6));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_eq!(get_stdout::<BasicMem>(p).unwrap(), &["1", "2", "3", "5", "6"]);
}

#[test]
/// In this test, `try_fn` does not unwind. Therefore `catch_fn` is not executed.
fn catch_no_unwind() {
    let mut p = ProgramBuilder::new();

    let try_fn = {
        let mut f = p.declare_function();
        f.print(const_int(2));
        f.return_();
        p.finish_function(f)
    };

    let catch_fn = {
        let mut f = p.declare_function();
        f.print(const_int(3));
        f.return_();
        p.finish_function(f)
    };

    let main_fn = {
        let mut f = p.declare_function();
        f.print(const_int(1));

        let cont = f.declare_block();

        let catch_block = f.catch_block(|f| {
            f.call_nounwind(unit_place(), fn_ptr(catch_fn), &[]);
            f.print(const_int(5));
            f.stop_unwind(cont);
        });
        f.call(unit_place(), fn_ptr(try_fn), &[], catch_block);
        f.print(const_int(4));
        f.goto(cont);
        f.set_cur_block(cont, BbKind::Regular);
        f.print(const_int(6));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_eq!(get_stdout::<BasicMem>(p).unwrap(), &["1", "2", "4", "6"]);
}
