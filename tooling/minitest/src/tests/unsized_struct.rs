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

    let f_ty = tuple_ty(
        &[(size(0), <u16>::get_type()), (size(8), <usize>::get_type())],
        size(16),
        align(8),
    );
    let g_ty =
        unsized_tuple_ty(&[(size(0), <u16>::get_type())], trait_obj_bar_ty, size(2), align(2));

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
