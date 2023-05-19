use crate::*;

/// This is a probabilistic test to check that the scheduler allows for multiple orderings.
/// The probability of failure is: 2*(1/2)^20 which is about 0.0002% with a fair scheduler.
#[test]
fn arbitrary_order() {

    /// A function that writes 1 to the global(1).
    fn write_1() -> Function {
        let b0 = block!(
            acquire(load(global::<u32>(0)), 1)
        );
        let b1 = block!(
            assign(global::<u32>(1), const_int::<u32>(1)),
            release(load(global::<u32>(0)), 2)
        );
        let b2 = block!(return_());

        function(Ret::No, 0, &[], &[b0, b1, b2])
    }

    // Main function, creates a lock and a thread.
    // It then tries to write 2 to global(1).

    // The locals are used to store the thread ids.
    let locals = [<u32>::get_ptype()];

    // Create the lock and store its id at global(0).
    let b0 = block!(
        create_lock(global::<u32>(0), 1)
    );

    // Spawn thread-1 and store its id at local(0).
    // The function given to it tries to write 1.
    let b1 = block!(
        storage_live(0),
        spawn(fn_ptr(1), Some(local(0)), 2)
    );

    // Write 2 to global(1) within critical section.
    let b2 = block!(
        acquire(load(global::<u32>(0)), 3)
    );

    let b3 = block!(
        assign(global::<u32>(1), const_int::<u32>(2)),
        release(load(global::<u32>(0)), 4)
    );

    // Join thread again.
    let b4 = block!(
        join(load(local(0)), 5)
    );

    // Print out the value last written to global(1).
    let b5 = block!(
        print(load(global::<u32>(1)), 6)
    );
    let b6 = block!( exit() );

    let f = function(Ret::No, 0, &locals, &[b0, b1, b2, b3, b4, b5, b6]);

    // global(0) is used as a lock. We store the lock id there.
    // global(1) is the place where both threads try to write to.
    let globals = [global_int::<u32>(), global_int::<u32>()];

    let p = program_with_globals(&[f, write_1()], &globals);
    
    // We now test, that the program can both finish with global(1) = 1 and = 2.
    let mut write_1 = false;
    let mut write_2 = false;

    for _ in 0..20 {
        let out = match get_out(p) {
            Ok(out) => out,
            Err(err) => panic!("{:?}", err),
        };

        if out[0] == "1" { write_1 = true; }
        if out[0] == "2" { write_2 = true; }
    }

    assert!( write_1 );
    assert!( write_2 );
}
