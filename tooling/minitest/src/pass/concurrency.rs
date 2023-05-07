use crate::*;

#[test]
fn arbitrary_order() {

    // A function that writes x to the global(1).
    fn write(x: u32) -> Function {
        let locals = [<u32>::get_ptype()];

        let b0 = block!(
            storage_live(0),
            assign(local(0), load(global::<u32>(0))),
            acquire(load(local(0)), 1)
        );
        let b1 = block!(
            assign(global::<u32>(1), const_int::<u32>(x)),
            release(load(local(0)), 2)
        );
        let b2 = block!(return_());

        function(Ret::No, 0, &locals, &[b0, b1, b2])
    }

    // Main function, creates a lock and two threads.
    let locals = [<u32>::get_ptype(), <u32>::get_ptype()];

    let b0 = block!(
        create_lock(global::<u32>(0), 1)
    );
    let b1 = block!(
        storage_live(0),
        spawn(fn_ptr(1), Some(local(0)), 2)
    );
    let b2 = block!(
        storage_live(1),
        spawn(fn_ptr(1), Some(local(1)), 3)
    );
    let b3 = block!(
        join(load(local(0)), 4)
    );
    let b4 = block!(
        join(load(local(1)), 5)
    );
    let b5 = block!(exit());

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2, b3, b4, b5]);

    let globals = [global_int::<u32>(), global_int::<u32>()];

    let p = program(&[f, write(1), write(2)], &globals);
    
    // We now test, that the program can both finish with global(1) = 1 and = 2.
    let mut write_1 = false;
    let mut write_2 = false;

    for _ in 0..50 {
        let Ok(m) = get_final_machine(p) else {
            panic!("Machine not valid!");
        };
        
        m.st
    }
}
