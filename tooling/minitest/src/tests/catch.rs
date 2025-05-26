use crate::*;

#[test]
fn catch() {
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
        let catch_block = f.catch_block(|f| {
            f.call_nounwind(unit_place(), fn_ptr(catch_fn), &[]);
            f.goto_regular_block();
            f.print(const_int(5));
            f.exit();
        });
        f.call(unit_place(),fn_ptr(try_fn), &[], catch_block);
        f.print(const_int(4));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main_fn);
    dump_program(p);
    assert_eq!(get_stdout::<BasicMem>(p).unwrap(), &["1", "2", "4"]);
}