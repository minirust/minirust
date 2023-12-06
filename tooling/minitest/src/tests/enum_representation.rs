use crate::*;

/// Ill-formed: the only variant has size 0, but the enum is size 1
#[test]
fn ill_sized_enum_variant() {
    let enum_ty = enum_ty(&[enum_variant(<()>::get_type(), &[])], Discriminator::Known(0.into()), 1, size(1), align(1));
    let locals = &[enum_ty];
    let stmts = &[];
    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}

/// Ill-formed: the two variants have different sizes
#[test]
fn inconsistently_sized_enum_variants() {
    let enum_ty = enum_ty(&[
            enum_variant(<()>::get_type(), &[(offset(1), 2)]),  // size 0
            enum_variant(<bool>::get_type(), &[]),              // size 1
        ], Discriminator::Invalid, 1, size(1), align(1));       // size 1
    let locals = &[enum_ty];
    let stmts = &[];
    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}

/// Ill-formed: no variants but discriminator returns variant 1
#[test]
fn ill_formed_discriminator() {
    let enum_ty = enum_ty(&[], Discriminator::Known(1.into()), 1, size(0), align(1));
    let locals = &[enum_ty];
    let stmts = &[];
    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}

/// Works: simple roundtrip for both variants of an enum like Option<bool>
#[test]
fn simple_two_variant_works() {
    let bool_var_ty = enum_variant(Type::Bool, &[]);
    let empty_var_data_ty = tuple_ty(&[], size(1), align(1)); // unit with size 1
    let empty_var_ty = enum_variant(empty_var_data_ty, &[(offset(0), 2)]);
    let discriminator = Discriminator::Branch {
        offset: offset(0),
        fallback: GcCow::new(Discriminator::Invalid),
        children: [
            (0, Discriminator::Known(0.into())),
            (1, Discriminator::Known(0.into())),
            (2, Discriminator::Known(1.into())),
        ].into_iter().collect()
    };
    let enum_ty = enum_ty(&[bool_var_ty, empty_var_ty], discriminator, 1, size(1), align(1));
    
    let locals = &[enum_ty];
    let statements = &[
        storage_live(0),
        assign(local(0), variant(1, tuple(&[], empty_var_data_ty), enum_ty)),
        assign(local(0), load(local(0))),
        assign(local(0), variant(0, const_bool(false), enum_ty)),
        assign(local(0), load(local(0))),
        storage_dead(0)
    ];
    let prog = small_program(locals, statements);
    assert_stop(prog)
}

/// UB: Loading an uninhabited enum is UB as such a value is impossible to produce
/// It is the discriminant computation that fails, as we start off with Discriminator::Invalid.
#[test]
fn loading_uninhabited_enum_is_ub() {
    let enum_ty = enum_ty(&[], Discriminator::Invalid, 1, size(0), align(1));
    let locals = &[enum_ty];
    let stmts = &[
        storage_live(0),
        assign(local(0), load(local(0))), // UB here.
    ];
    let prog = small_program(locals, stmts);
    assert_ub(prog, "load at type Enum { variants: List([]), discriminator: Invalid, discriminant_ty: IntType { signed: Unsigned, size: Size(1 bytes) }, size: Size(0 bytes), align: Align(1 bytes) } but the data in memory violates the validity invariant");
}

/// Ill-formed: trying to build a variant value of an uninhabited enum
#[test]
fn ill_formed_variant_constant() {
    let enum_ty = enum_ty(&[], Discriminator::Invalid, 1, size(0), align(1));
    let locals = &[enum_ty];
    let stmts = &[
        storage_live(0),
        assign(local(0), variant(0, unit(), enum_ty)), // ill-formed here
    ];
    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}

/// Ill-formed: The data of the variant value does not match the type
#[test]
fn ill_formed_variant_constant_data() {
    let enum_ty = enum_ty(&[enum_variant(<u8>::get_type(), &[])], Discriminator::Known(0.into()), 1, size(1), align(1));
    let locals = &[enum_ty];
    let stmts = &[
        storage_live(0),
        assign(local(0), variant(1, unit(), enum_ty)), // ill-formed here
    ];
    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}

