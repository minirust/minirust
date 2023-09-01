use crate::*;

/// Test that `*dangling_ref` is UB even if the place is not loaded from.
#[test]
fn deref_dangling_ref() {
    let locals = [ <*const i32>::get_type() ];
    let dangling_ref = transmute(const_int(16usize), <&i32>::get_type());
    let b0 = block!(
        storage_live(0),
        assign(
            local(0),
            // We even deref to a `()` place, but the reference type is what matters.
            addr_of(deref(dangling_ref, <()>::get_type()), <*const i32>::get_type()),
        ),
        exit(),
    );
    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    assert_ub(p, "non-zero-sized access with invalid pointer");
}

/// Test that handling a dangling reference is allowed as long as we don't dereference it.
/// IOW, being dereferenceable is not part of the reference's validity requirements.
#[test]
fn assign_dangling_ref() {
    let locals = [ <&i32>::get_type() ];
    let dangling_ref = transmute(const_int(16usize), <&i32>::get_type());
    let b0 = block!(
        storage_live(0),
        assign(local(0), dangling_ref),
        exit(),
    );
    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    assert_stop(p);
}

/// However, when actually *validating* the reference, it will complain.
#[test]
fn validate_dangling_ref() {
    let locals = [ <&i32>::get_type() ];
    let dangling_ref = transmute(const_int(16usize), <&i32>::get_type());
    let b0 = block!(
        storage_live(0),
        assign(local(0), dangling_ref),
        validate(local(0), /* fn_entry */ false),
        exit(),
    );
    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    assert_ub(p, "non-zero-sized access with invalid pointer");
}

/// Test that `*dangling_ptr` is allowed as long as we don't do anything with that place.
#[test]
fn deref_dangling_ptr() {
    let locals = [ <*const i32>::get_type() ];
    let dangling_ptr = transmute(const_int(16usize), <*const i32>::get_type());
    let b0 = block!(
        storage_live(0),
        assign(
            local(0),
            addr_of(deref(dangling_ptr, <i32>::get_type()), <*const i32>::get_type()),
        ),
        exit(),
    );
    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    assert_stop(p);
}

/// Test that `&*dangling_ptr` is detected.
#[test]
fn ref_dangling_ptr() {
    let locals = [ <&i32>::get_type() ];
    let dangling_ptr = transmute(const_int(16usize), <*const i32>::get_type());
    let b0 = block!(
        storage_live(0),
        assign(
            local(0),
            // We make the place a 1-ZST so it's nothing about the place that complains here.
            addr_of(deref(dangling_ptr, <()>::get_type()), <&i32>::get_type()),
        ),
        exit(),
    );
    let f = function(Ret::No, 0, &locals, &[b0]);
    let p = program(&[f]);
    assert_ub(p, "non-zero-sized access with invalid pointer");
}
