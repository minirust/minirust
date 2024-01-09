use crate::*;

const U8_INTTYPE: IntType = IntType { signed: Signedness::Unsigned, size: Size::from_bytes_const(1) };

/// Ill-formed: the only variant has size 0, but the enum is size 1
#[test]
fn ill_sized_enum_variant() {
    let enum_ty = enum_ty::<u8>(&[enum_variant(<()>::get_type(), &[])], Discriminator::Known(0.into()), size(1), align(1));
    let locals = &[enum_ty];
    let stmts = &[];
    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}

/// Ill-formed: the two variants have different sizes
#[test]
fn inconsistently_sized_enum_variants() {
    let enum_ty = enum_ty::<u8>(&[
            enum_variant(<()>::get_type(), &[(offset(1), (U8_INTTYPE, 2.into()))]),  // size 0
            enum_variant(<bool>::get_type(), &[]),              // size 1
        ], Discriminator::Invalid, size(1), align(1));       // size 1
    let locals = &[enum_ty];
    let stmts = &[];
    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}

/// Ill-formed: no variants but discriminator returns variant 1
#[test]
fn ill_formed_discriminator() {
    let enum_ty = enum_ty::<u8>(&[], Discriminator::Known(1.into()), size(0), align(1));
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
    let empty_var_ty = enum_variant(empty_var_data_ty, &[(offset(0), (U8_INTTYPE, 2.into()))]);
    let discriminator = Discriminator::Branch {
        offset: offset(0),
        fallback: GcCow::new(Discriminator::Invalid),
        value_type: U8_INTTYPE,
        children: [
            (0.into(), Discriminator::Known(0.into())),
            (1.into(), Discriminator::Known(0.into())),
            (2.into(), Discriminator::Known(1.into())),
        ].into_iter().collect()
    };
    let enum_ty = enum_ty::<u8>(&[bool_var_ty, empty_var_ty], discriminator, size(1), align(1));
    
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
    let enum_ty = enum_ty::<u8>(&[], Discriminator::Invalid, size(0), align(1));
    let locals = &[enum_ty];
    let stmts = &[
        storage_live(0),
        assign(local(0), load(local(0))), // UB here.
    ];
    let prog = small_program(locals, stmts);
    assert_ub(prog, "load at type Enum { variants: List([]), discriminant_ty: IntType { signed: Unsigned, size: Size(1 bytes) }, discriminator: Invalid, size: Size(0 bytes), align: Align(1 bytes) } but the data in memory violates the validity invariant");
}

/// Ill-formed: trying to build a variant value of an uninhabited enum
#[test]
fn ill_formed_variant_constant() {
    let enum_ty = enum_ty::<u8>(&[], Discriminator::Invalid, size(0), align(1));
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
    let enum_ty = enum_ty::<u8>(&[enum_variant(<u8>::get_type(), &[])], Discriminator::Known(0.into()), size(1), align(1));
    let locals = &[enum_ty];
    let stmts = &[
        storage_live(0),
        assign(local(0), variant(1, unit(), enum_ty)), // ill-formed here
    ];
    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}


/// Ill-formed: Ensures that the enum alignment is at least as big as all the variant alignments.
#[test]
fn ill_formed_enum_must_have_maximal_alignment_of_inner() {
    let enum_ty = enum_ty::<u8>(&[enum_variant(<u16>::get_type(), &[])], Discriminator::Known(0.into()), size(2), align(1));
    let locals = [enum_ty];
    let stmts = [];
    let prog = small_program(&locals, &stmts);
    assert_ill_formed(prog);
}

const U32_INTTYPE: IntType = IntType { signed: Signedness::Unsigned, size: Size::from_bytes_const(4) };

/// Works: Tests that using a tag other than u8 works, here using a u32.
#[test]
fn larger_sized_tag_works() {
    let variant_0_tuple_ty = tuple_ty(&[(offset(0), <u32>::get_type())], size(8), align(4));
    let enum_ty = enum_ty::<u8>(
        &[enum_variant(variant_0_tuple_ty, &[(offset(4), (U32_INTTYPE, 1048576.into()))])],
        Discriminator::Branch {
            offset: offset(4),
            value_type: U32_INTTYPE,
            fallback: GcCow::new(Discriminator::Invalid),
            children: [(1048576.into(), Discriminator::Known(0.into()))].into_iter().collect()
        },
        size(8),
        align(4)
    );

    let locals = &[enum_ty];
    let statements = &[
        storage_live(0),
        assign(local(0), variant(0, tuple(&[const_int(2774879812u32)], variant_0_tuple_ty), enum_ty)),
        assign(local(0), load(local(0))),
        storage_dead(0)
    ];
    let prog = small_program(locals, statements);
    assert_stop(prog)
}

/// Works: Tests that using a tag larger than u8 has no alignment requirements.
#[test]
fn larger_tag_has_no_alignment() {
    let variant_0_tuple_ty = tuple_ty(&[(offset(0), <u32>::get_type())], size(12), align(4));
    let enum_ty = enum_ty::<u8>(
        &[enum_variant(variant_0_tuple_ty, &[(offset(6), (U32_INTTYPE, 1048576.into()))])],
        Discriminator::Branch {
            offset: offset(6),
            value_type: U32_INTTYPE,
            fallback: GcCow::new(Discriminator::Invalid),
            children: [(1048576.into(), Discriminator::Known(0.into()))].into_iter().collect()
        },
        size(12),
        align(4)
    );

    let locals = &[enum_ty];
    let statements = &[
        storage_live(0),
        assign(local(0), variant(0, tuple(&[const_int(2774879812u32)], variant_0_tuple_ty), enum_ty)),
        assign(local(0), load(local(0))),
        storage_dead(0)
    ];
    let prog = small_program(locals, statements);
    assert_stop(prog)
}
