use crate::*;

/// Models the following example:
/// ```rust
/// trait A {
///     fn foo(&self) -> usize;
/// }
///
/// impl A for usize {
///     fn foo(&self) -> usize {
///         *self
///     }
/// }
///
/// let x: usize = 42;
/// let y: &dyn A = &x;
/// assert_eq!(x == y.foo());
/// ```
#[test]
fn dynamic_dispatch() {
    let mut p = ProgramBuilder::new();

    let mut trait_a = p.declare_trait();
    let method_a_foo = trait_a.declare_method();
    let trait_a = trait_a.finish_trait();
    let trait_obj_a_ty = trait_object_ty(trait_a);

    let impl_a_foo_for_usize = {
        let mut f = p.declare_function();

        let self_ = f.declare_arg::<&usize>();
        let ret = f.declare_ret::<usize>();
        f.assign(ret, load(deref(load(self_), <usize>::get_type())));
        f.return_();

        p.finish_function(f)
    };

    let mut usize_a_vtable = p.declare_vtable_for_ty(trait_a, <usize>::get_type());
    usize_a_vtable.add_method(method_a_foo, impl_a_foo_for_usize);
    let usize_a_vtable = p.finish_vtable(usize_a_vtable);

    let main = {
        let mut main = p.declare_function();

        let x = main.declare_local::<usize>();
        main.storage_live(x);
        main.assign(x, const_int(42_usize));

        let y = main.declare_local_with_ty(ref_ty_for(trait_obj_a_ty));
        let y_val = construct_wide_pointer(
            addr_of(x, <&usize>::get_type()),
            ValueExpr::Constant(
                Constant::VTablePointer(usize_a_vtable),
                Type::Ptr(PtrType::VTablePtr),
            ),
            ref_ty_for(trait_obj_a_ty),
        );
        main.storage_live(y);
        main.assign(y, y_val);

        let foo_ret = main.declare_local::<usize>();
        main.storage_live(foo_ret);
        main.call(
            foo_ret,
            vtable_lookup(get_metadata(load(y)), method_a_foo),
            &[by_value(ptr_to_ptr(get_thin_pointer(load(y)), <&usize>::get_type()))],
        );
        main.assume(eq(load(x), load(foo_ret)));

        main.exit();
        p.finish_function(main)
    };

    let p = p.finish_program(main);
    dump_program(p);
    assert_stop::<BasicMem>(p);
}
