use crate::*;

enum AccessType {
    Load,
    Store,
}

struct AccessPattern(AccessType, Atomicity);

// A block that does the access pattern on global(0): stores put the value of the "support global"
// into global(0); loads pit the value of global(0) into the "support global".
fn access_block(access: AccessPattern, support_global: u32, next: u32) -> BasicBlock {
    let ptr_ty = raw_ptr_ty();
    let addr = addr_of(global::<u32>(0), ptr_ty);
    match access {
        AccessPattern(AccessType::Load, Atomicity::Atomic) => {
            block!(
                atomic_load(global::<u32>(support_global), addr, next)
            )
        },
        AccessPattern(AccessType::Load, Atomicity::None) => {
            block!(
                assign(global::<u32>(support_global), load(global::<u32>(0))),
                goto(next),
            )
        },
        AccessPattern(AccessType::Store, Atomicity::Atomic) => {
            block!(
                atomic_store(addr, load(global::<u32>(support_global)), next)
            )
        },
        AccessPattern(AccessType::Store, Atomicity::None) => {
            block!(
                assign(global::<u32>(0), load(global::<u32>(support_global))),
                goto(next),
            )
        }
    }
}

fn racy_program(main_access: AccessPattern, s_access: AccessPattern) -> Program {
    // The main thread.
    let main_locals = [<u32>::get_type()];

    let main_b0 = block!(
        storage_live(0),
        spawn(fn_ptr(1), null(), local(0), 1),
    );
    let main_b1 = access_block(main_access, 1, 2);
    let main_b2 = block!(
        join(load(local(0)), 3),
    );
    let main_b3 = block!( exit() );
    let main = function(Ret::No, 0, &main_locals, &[main_b0, main_b1, main_b2, main_b3]);

    // The second thread.
    let s_locals = [<()>::get_type(), <*const ()>::get_type()];
    let s_b0 = access_block(s_access, 2, 1);
    let s_b1 = block!(
        return_()
    );
    let s_fun = function(Ret::Yes, 1, &s_locals, &[s_b0, s_b1]);

    // global(0) is needed for the race behavior; the others are used to support our operations.
    // We use globals instead of locals because locals would need an additional instruction (`storage_live`)
    // before the race condition which would decrease the chance of it being caught.
    let globals = [global_int::<u32>(); 3];

    program_with_globals(&[main, s_fun], &globals)
}

#[test]
fn atomic_load_atomic_load() {
    let p = racy_program(
        AccessPattern(AccessType::Load, Atomicity::Atomic),
        AccessPattern(AccessType::Load, Atomicity::Atomic)
    );

    assert!(!has_data_race(p))
}

#[test]
fn atomic_load_atomic_store() {
    let p = racy_program(
        AccessPattern(AccessType::Load, Atomicity::Atomic),
        AccessPattern(AccessType::Store, Atomicity::Atomic)
    );

    assert!(!has_data_race(p))
}

#[test]
fn atomic_load_non_atomic_load() {
    let p = racy_program(
        AccessPattern(AccessType::Load, Atomicity::Atomic),
        AccessPattern(AccessType::Load, Atomicity::None)
    );

    assert!(!has_data_race(p))
}

#[test]
fn atomic_load_non_atomic_store() {
    let p = racy_program(
        AccessPattern(AccessType::Load, Atomicity::Atomic),
        AccessPattern(AccessType::Store, Atomicity::None)
    );

    assert!(has_data_race(p))
}

#[test]
fn atomic_store_atomic_store() {
    let p = racy_program(
        AccessPattern(AccessType::Store, Atomicity::Atomic),
        AccessPattern(AccessType::Store, Atomicity::Atomic)
    );

    assert!(!has_data_race(p))
}

#[test]
fn atomic_store_non_atomic_load() {
    let p = racy_program(
        AccessPattern(AccessType::Store, Atomicity::Atomic),
        AccessPattern(AccessType::Load, Atomicity::None)
    );

    assert!(has_data_race(p))
}

#[test]
fn atomic_store_non_atomic_store() {
    let p = racy_program(
        AccessPattern(AccessType::Store, Atomicity::Atomic),
        AccessPattern(AccessType::Store, Atomicity::None)
    );

    assert!(has_data_race(p))
}

#[test]
fn non_atomic_load_non_atomic_load() {
    let p = racy_program(
        AccessPattern(AccessType::Load, Atomicity::None),
        AccessPattern(AccessType::Load, Atomicity::None)
    );

    assert!(!has_data_race(p))
}

#[test]
fn non_atomic_load_non_atomic_store() {
    let p = racy_program(
        AccessPattern(AccessType::Load, Atomicity::None),
        AccessPattern(AccessType::Store, Atomicity::None)
    );

    assert!(has_data_race(p))
}

#[test]
fn non_atomic_store_non_atomic_store() {
    let p = racy_program(
        AccessPattern(AccessType::Store, Atomicity::None),
        AccessPattern(AccessType::Store, Atomicity::None)
    );

    assert!(has_data_race(p))
}
