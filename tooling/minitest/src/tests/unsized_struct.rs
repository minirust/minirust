use crate::*;

/// Models the following (shortened) Miri test:
/// ```rust
/// /// This struct will be a DST iff T is a DST
/// struct Foo<T: ?Sized> {
///     a: u16,
///     b: T,
/// }
///
/// /// A trait to test trait objects
/// trait Bar {
///     fn get(&self) -> usize;
/// }
/// impl Bar for usize {
///     fn get(&self) -> usize {
///         *self
///     }
/// }
///
/// let f: Foo<usize> = Foo { a: 0, b: 11 };
/// // usize is cast to dyn Bar in inner, needs to be behind pointer
/// let g: &Foo<dyn Bar> = &f;
/// assert_eq!(g.b.get(), 11);
/// assert_eq!(core::mem::size_of_val(g), 16);
/// assert_eq!(core::mem::align_of_val(g), 8);
/// ```
#[test]
fn unsized_tail() {
    let mut p = ProgramBuilder::new();

    let mut trait_bar = p.declare_trait();
    let method_bar_get = trait_bar.declare_method();
    let trait_bar = p.finish_trait(trait_bar);
    let trait_obj_bar_ty = trait_object_ty(trait_bar);

    let impl_bar_get_for_usize = {
        let mut f = p.declare_function();

        let self_ = f.declare_arg::<&usize>();
        let ret = f.declare_ret::<usize>();
        f.assign(ret, load(deref(load(self_), <usize>::get_type())));
        f.return_();

        p.finish_function(f)
    };

    let mut usize_bar_vtable = p.declare_vtable_for_ty(trait_bar, <usize>::get_type());
    usize_bar_vtable.add_method(method_bar_get, impl_bar_get_for_usize);
    let usize_bar_vtable = p.finish_vtable(usize_bar_vtable);

    // type `(u16, usize)`
    let f_ty = tuple_ty(
        &[(size(0), <u16>::get_type()), (size(8), <usize>::get_type())],
        size(16),
        align(8),
    );
    // type `(u16, dyn Bar)`
    let g_ty = unsized_tuple_ty(
        &[(size(0), <u16>::get_type())],
        trait_obj_bar_ty,
        size(2),
        align(2),
        None,
    );

    let main = {
        let mut main = p.declare_function();

        // `let f: Foo<usize> = Foo { a: 0, b: 11 };`
        let f = main.declare_local_with_ty(f_ty);
        main.storage_live(f);
        main.assign(f, tuple(&[const_int(0_u16), const_int(11_usize)], f_ty));

        // `let g: &Foo<dyn Bar> = &f;`
        let g = main.declare_local_with_ty(ref_ty_default_markers_for(g_ty));
        let g_val = construct_wide_pointer(
            addr_of(f, ref_ty_default_markers_for(f_ty)),
            const_vtable(usize_bar_vtable),
            ref_ty_default_markers_for(g_ty),
        );
        main.storage_live(g);
        main.assign(g, g_val);

        // `let g_b: &dyn Bar = &g.b;`
        let g_b = main.declare_local_with_ty(ref_mut_ty_default_markers_for(trait_obj_bar_ty));
        main.storage_live(g_b);
        main.assign(
            g_b,
            addr_of(
                field(deref(load(g), g_ty), 1),
                ref_mut_ty_default_markers_for(trait_obj_bar_ty),
            ),
        );

        // `assert_eq!(g_b.get(), 11);`
        let get_ret = main.declare_local::<usize>();
        main.storage_live(get_ret);
        main.call(get_ret, vtable_lookup(get_metadata(load(g_b)), method_bar_get), &[by_value(
            get_thin_pointer(load(g_b)),
        )]);
        main.assume(eq(const_int(11_usize), load(get_ret)));

        // `assert_eq!(core::mem::size_of_val(g), 16);`
        // `assert_eq!(core::mem::align_of_val(g), 8);`
        main.assume(eq(const_int(16_usize), compute_size(g_ty, get_metadata(load(g)))));
        main.assume(eq(const_int(8_usize), compute_align(g_ty, get_metadata(load(g)))));

        main.exit();
        p.finish_function(main)
    };

    let p = p.finish_program(main);
    dump_program(p);
    assert_stop::<BasicMem>(p);
}

/// Models the following (shortened) Rustc test:
/// <tests/ui/packed/dyn-trait.rs>
/// ```rust
/// use std::ptr::addr_of;
///
/// // When the unsized tail is a `dyn Trait`, its alignments is only dynamically known. This means the
/// // packed(2) needs to be applied at runtime: the actual alignment of the field is `min(2,
/// // usual_alignment)`. Here we check that we do this right by comparing size, alignment, and field
/// // offset before and after unsizing.
/// fn main() {
///     #[repr(C, packed(2))]
///     struct Packed<T: ?Sized>(u8, T);
///
///     let s = Packed(0, 1);
///     let p: &Packed<usize> = &s;
///     let sized = (core::mem::size_of_val(p), core::mem::align_of_val(p));
///     let sized_offset = unsafe { addr_of!(p.1).cast::<u8>().offset_from(addr_of!(p.0)) };
///     let q: &Packed<dyn Send> = p;
///     let un_sized = (core::mem::size_of_val(q), core::mem::align_of_val(q));
///     let un_sized_offset = unsafe { addr_of!(q.1).cast::<u8>().offset_from(addr_of!(q.0)) };
///     assert_eq!(sized, un_sized);
///     assert_eq!(sized_offset, un_sized_offset);
/// }
/// ```
#[test]
fn packed_tail() {
    let mut prog = ProgramBuilder::new();

    let trait_send = prog.declare_trait();
    let trait_send = prog.finish_trait(trait_send);
    let trait_obj_send_ty = trait_object_ty(trait_send);

    let usize_send_vtable = prog.declare_vtable_for_ty(trait_send, <usize>::get_type());
    let usize_send_vtable = prog.finish_vtable(usize_send_vtable);

    // type `#packed(2) (u8, usize)`
    let p_ty = tuple_ty(
        &[(size(0), <u8>::get_type()), (size(2), <usize>::get_type())],
        size(10),
        align(2),
    );
    // type `#packed(2) (u8, dyn Send)`
    let q_ty = unsized_tuple_ty(
        &[(size(0), <u8>::get_type())],
        trait_obj_send_ty,
        size(1),
        align(1),
        Some(align(2)),
    );

    let main = {
        let mut f = prog.declare_function();

        // `let s = Packed(0, 1);`
        let s = f.declare_local_with_ty(p_ty);
        f.storage_live(s);
        f.assign(s, tuple(&[const_int(0_u8), const_int(1_usize)], p_ty));

        // `let p: &Packed<usize> = &s;`
        let p = f.declare_local_with_ty(ref_ty_default_markers_for(p_ty));
        f.storage_live(p);
        f.assign(p, addr_of(s, ref_ty_default_markers_for(p_ty)));

        // `let q: &Packed<dyn Send> = p;`
        let q = f.declare_local_with_ty(ref_ty_default_markers_for(q_ty));
        let q_val = construct_wide_pointer(
            load(p),
            const_vtable(usize_send_vtable),
            ref_ty_default_markers_for(q_ty),
        );
        f.storage_live(q);
        f.assign(q, q_val);

        // `let sized = (core::mem::size_of_val(p), core::mem::align_of_val(p));` ...
        // `assert_eq!(sized, un_sized);`
        f.assume(eq(
            compute_size(p_ty, get_metadata(load(p))),
            compute_size(q_ty, get_metadata(load(q))),
        ));
        f.assume(eq(
            compute_align(p_ty, get_metadata(load(p))),
            compute_align(q_ty, get_metadata(load(q))),
        ));

        // `assert_eq!(addr_of!(p.1), addr_of!(q.1));`
        f.assume(eq(
            addr_of(field(deref(load(p), p_ty), 1), <*const usize>::get_type()),
            get_thin_pointer(addr_of(
                field(deref(load(q), q_ty), 1),
                raw_ptr_ty(PointerMetaKind::VTablePointer(trait_send)),
            )),
        ));

        f.exit();
        prog.finish_function(f)
    };

    let p = prog.finish_program(main);
    assert_stop::<BasicMem>(p);
}

/// Checks the size and alignment of a nested unsized struct, with an outer packed attribute:
///
/// ```rust
/// #[repr(packed(2))]
/// struct Bar {
///     a: u16,
///     b: (),
///     c: Foo,
/// }
/// struct Foo {
///     d: u32,
///     e: [u64]
/// }
/// ```
#[test]
fn size_of_nested_struct() {
    let mut p = ProgramBuilder::new();

    // 0     2     4           8          12            12+8i
    // | u16 |  -  |    u32    |    ---    |   [u64] ...  |
    let nested_ty = unsized_tuple_ty(
        &[(size(0), <u16>::get_type()), (size(2), <()>::get_type())],
        unsized_tuple_ty(
            &[(size(0), <u32>::get_type())],
            <[u64]>::get_type(),
            size(4),
            align(4),
            None,
        ),
        size(2),
        align(2),
        Some(align(4)),
    );

    let main = {
        let mut f = p.declare_function();
        f.assume(eq(compute_size(nested_ty, const_int(0_usize)), const_int(12_usize)));
        f.assume(eq(compute_size(nested_ty, const_int(1_usize)), const_int(20_usize)));
        f.assume(eq(compute_align(nested_ty, const_int(0_usize)), const_int(4_usize)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main);
    assert_stop::<BasicMem>(p);
}

/// The alignment of the sized fields must not exceed the packed attribute
#[test]
fn ill_align_larger_than_packed() {
    let mut p = ProgramBuilder::new();

    let ill_ty = unsized_tuple_ty(
        &[(size(0), <u16>::get_type())],
        <[u16]>::get_type(),
        size(2),
        align(2),
        Some(align(1)),
    );

    let main = {
        let mut f = p.declare_function();
        f.print(compute_size(ill_ty, const_int(1_usize)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main);
    assert_ill_formed::<BasicMem>(p, "TupleHeadLayout: align bigger than packed attribute");
}

/// Setting a packed attribute is only relevant for unsized tail computations, thus a sized struct must not set this.
#[test]
fn ill_packed_on_sized() {
    let mut p = ProgramBuilder::new();

    let ill_ty = Type::Tuple {
        sized_fields: List::new(),
        sized_head_layout: TupleHeadLayout {
            end: size(0),
            align: align(1),
            packed_align: Some(align(1)),
        },
        unsized_field: GcCow::new(None),
    };

    let main = {
        let mut f = p.declare_function();
        f.print(compute_size(ill_ty, const_int(1_usize)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main);
    assert_ill_formed::<BasicMem>(p, "Type::Tuple: meaningless packed align for sized tuple");
}

/// The fields in the tuple head must be sized
#[test]
fn ill_unsized_head() {
    let mut p = ProgramBuilder::new();

    let ill_ty = tuple_ty(&[(size(0), <[u16]>::get_type())], size(2), align(2));

    let main = {
        let mut f = p.declare_function();
        f.print(compute_size(ill_ty, const_int(1_usize)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main);
    assert_ill_formed::<BasicMem>(p, "Type::Tuple: unsized field type in head");
}

/// The unsized tail of a tuple must not be sized.
#[test]
fn ill_sized_tail() {
    let mut p = ProgramBuilder::new();

    let ill_ty = unsized_tuple_ty(
        &[(size(0), <u16>::get_type())],
        <u32>::get_type(),
        size(2),
        align(2),
        None,
    );

    let main = {
        let mut f = p.declare_function();
        f.print(compute_size(ill_ty, const_int(1_usize)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main);
    assert_ill_formed::<BasicMem>(p, "Type::Tuple: sized unsized field type");
}

/// The end of a tuple must be after every field.
#[test]
fn ill_size_too_small() {
    let mut p = ProgramBuilder::new();

    let ill_ty = tuple_ty(
        &[(size(0), <u64>::get_type()), (size(8), <()>::get_type()), (size(8), <u32>::get_type())],
        size(10),
        align(2),
    );

    let main = {
        let mut f = p.declare_function();
        f.print(compute_size(ill_ty, const_int(1_usize)));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main);
    assert_ill_formed::<BasicMem>(
        p,
        "Type::Tuple: size of fields is bigger than the end of the sized head",
    );
}

/// An aggregate operation can only work with sized tuples.
#[test]
fn ill_aggregate_unsized() {
    let mut p = ProgramBuilder::new();

    // type `(u16, [u8])`
    let ty = unsized_tuple_ty(
        &[(size(0), <u16>::get_type())],
        <[u8]>::get_type(),
        size(2),
        align(2),
        None,
    );

    let main = {
        let mut f = p.declare_function();
        let x =
            f.declare_local_with_ty(tuple_ty(&[(size(0), <u16>::get_type())], size(2), align(2)));
        f.storage_live(x);
        f.assign(x, tuple(&[const_int(4_u16)], ty));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main);
    assert_ill_formed::<BasicMem>(p, "ValueExpr::Tuple: constructing an unsized tuple value");
}
