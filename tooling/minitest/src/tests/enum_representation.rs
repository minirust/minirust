use crate::*;

const U8_INTTYPE: IntType = IntType { signed: Signedness::Unsigned, size: Size::from_bytes_const(1) };

/// Ill-formed: the only variant has size 0, but the enum is size 1
#[test]
fn ill_sized_enum_variant() {
    let enum_ty = enum_ty::<u8>(&[(0, enum_variant(<()>::get_type(), &[]))], discriminator_known(0), size(1), align(1));
    let locals = &[enum_ty];
    let stmts = &[];
    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}

/// Ill-formed: the two variants have different sizes
#[test]
fn inconsistently_sized_enum_variants() {
    let enum_ty = enum_ty::<u8>(&[
            (0, enum_variant(<()>::get_type(), &[(offset(1), (U8_INTTYPE, 2.into()))])),  // size 0
            (1, enum_variant(<bool>::get_type(), &[])),                                   // size 1
        ], discriminator_invalid(), size(1), align(1));                                   // size 1
    let locals = &[enum_ty];
    let stmts = &[];
    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}

/// Ill-formed: no variants but discriminator returns variant 1
#[test]
fn ill_formed_discriminator() {
    let enum_ty = enum_ty::<u8>(&[], discriminator_known(1), size(0), align(1));
    let locals = &[enum_ty];
    let stmts = &[];
    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}

/// Ill-formed: discriminator branch has a case of -1 which is an invalid value for u8. 
#[test]
fn ill_formed_discriminator_branch() {
    // enum based on Option<NonZeroU8>.
    let enum_ty = enum_ty::<u8>(&[
            (0, enum_variant(<u8>::get_type(), &[])),
            (1, enum_variant(tuple_ty(&[], size(1), align(1)), &[(offset(0), (U8_INTTYPE, 0.into()))])),
        ],
        Discriminator::Branch {
            offset: offset(0),
            value_type: U8_INTTYPE,
            fallback: GcCow::new(discriminator_known(0)),
            children: [((Int::from(-1), Int::ZERO), discriminator_known(1))].into_iter().collect() // ill-formed here
        },
        size(1),
        align(1)
    );
    let locals = &[enum_ty];
    let stmts = &[];
    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}

/// Ill-formed: the discriminator branch children overlap.
#[test]
fn ill_formed_discriminator_overlaps() {
    let dataless_ty = tuple_ty(&[], size(1), align(1));
    let enum_ty = enum_ty::<u8>(&[
            (0, enum_variant(dataless_ty, &[])),
            (1, enum_variant(dataless_ty, &[])),
        ],
        discriminator_branch::<u8>(
            offset(0),
            discriminator_known(0),
            &[
                ((2.into(), 4.into()), discriminator_known(1)),
                ((1.into(), 5.into()), discriminator_known(0))  // ill-formed here
            ]
        ),
        size(1),
        align(1)
    );
    let locals = &[enum_ty];
    let stmts = &[];
    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}

/// Ill-formed: the discriminator branch children overlap.
#[test]
fn ill_formed_discriminator_overlaps_2() {
    let dataless_ty = tuple_ty(&[], size(1), align(1));
    let enum_ty = enum_ty::<u8>(&[
            (0, enum_variant(dataless_ty, &[])),
            (1, enum_variant(dataless_ty, &[])),
        ],
        discriminator_branch::<u8>(
            offset(0),
            discriminator_known(0),
            &[
                ((2, 4), discriminator_known(1)),
                ((1, 3), discriminator_known(0))  // ill-formed here
            ]
        ),
        size(1),
        align(1)
    );
    let locals = &[enum_ty];
    let stmts = &[];
    let prog = small_program(locals, stmts);
    assert_ill_formed(prog);
}

/// Ill-formed: discriminant is of type u8 but variant has discriminant -1.
#[test]
fn ill_formed_discriminant_value() {
    let enum_ty = Type::Enum {
        variants: [(Int::from(-1), enum_variant(<u8>::get_type(), &[]))].into_iter().collect(),
        discriminant_ty: U8_INTTYPE,
        discriminator: discriminator_known(-1),
        size: size(1),
        align: align(1),
    };
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
    let discriminator = discriminator_branch::<u8>(
        offset(0),
        discriminator_invalid(),
        &[
            ((0, 1), discriminator_known(0)),
            ((1, 2), discriminator_known(0)),
            ((2, 3), discriminator_known(1)),
        ]
    );
    let enum_ty = enum_ty::<u8>(&[(0, bool_var_ty), (1, empty_var_ty)], discriminator, size(1), align(1));
    
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
    let enum_ty = enum_ty::<u8>(&[], discriminator_invalid(), size(0), align(1));
    let locals = &[enum_ty];
    let stmts = &[
        storage_live(0),
        assign(local(0), load(local(0))), // UB here.
    ];
    let prog = small_program(locals, stmts);
    assert_ub(prog, "load at type Enum { variants: Map({}), discriminant_ty: IntType { signed: Unsigned, size: Size(1 bytes) }, discriminator: Invalid, size: Size(0 bytes), align: Align(1 bytes) } but the data in memory violates the validity invariant");
}

/// Ill-formed: trying to build a variant value of an uninhabited enum
#[test]
fn ill_formed_variant_constant() {
    let enum_ty = enum_ty::<u8>(&[], discriminator_invalid(), size(0), align(1));
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
    let enum_ty = enum_ty::<u8>(&[(0, enum_variant(<u8>::get_type(), &[]))], discriminator_known(0), size(1), align(1));
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
    let enum_ty = enum_ty::<u8>(&[(0, enum_variant(<u16>::get_type(), &[]))], discriminator_known(0), size(2), align(1));
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
    let enum_ty = enum_ty::<u32>(
        &[(1048576, enum_variant(variant_0_tuple_ty, &[(offset(4), (U32_INTTYPE, 1048576.into()))]))],
        discriminator_branch::<u32>(
            offset(4),
            discriminator_invalid(),
            &[((1048576, 1048577), discriminator_known(1048576))]
        ),
        size(8),
        align(4)
    );

    let locals = &[enum_ty];
    let statements = &[
        storage_live(0),
        assign(local(0), variant(1048576, tuple(&[const_int(2774879812u32)], variant_0_tuple_ty), enum_ty)),
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
    let enum_ty = enum_ty::<u32>(
        &[(1048576, enum_variant(variant_0_tuple_ty, &[(offset(6), (U32_INTTYPE, 1048576.into()))]))],
        discriminator_branch::<u32>(
            offset(6),
            discriminator_invalid(),
            &[((1048576, 1048577), discriminator_known(1048576))]
        ),
        size(12),
        align(4)
    );

    let locals = &[enum_ty];
    let statements = &[
        storage_live(0),
        assign(local(0), variant(1048576, tuple(&[const_int(2774879812u32)], variant_0_tuple_ty), enum_ty)),
        assign(local(0), load(local(0))),
        storage_dead(0)
    ];
    let prog = small_program(locals, statements);
    assert_stop(prog)
}

/// Works: tests that negative discriminants are valid.
#[test]
fn negative_discriminants_work() {
    let i16_it = IntType { size: size(2), signed: Signedness::Signed };
    let variant_0_tuple_ty = tuple_ty(&[(offset(0), <u32>::get_type())], size(8), align(4));
    let enum_ty = enum_ty::<i16>(
        &[(i16::MAX, enum_variant(variant_0_tuple_ty, &[(offset(4), (i16_it, (-12989).into()))]))],
        discriminator_branch::<i16>(
            offset(4),
            discriminator_invalid(),
            &[((-12989, -12988), discriminator_known(i16::MAX))]
        ),
        size(8),
        align(4)
    );

    let locals = &[enum_ty];
    let statements = &[
        storage_live(0),
        assign(local(0), variant(i16::MAX, tuple(&[const_int(2774879812u32)], variant_0_tuple_ty), enum_ty)),
        assign(local(0), load(local(0))),
        storage_dead(0)
    ];
    let prog = small_program(locals, statements);
    assert_stop(prog)
}
