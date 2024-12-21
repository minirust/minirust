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
    let trait_a = p.finish_trait(trait_a);
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

        let y = main.declare_local_with_ty(ref_ty_default_markers_for(trait_obj_a_ty));
        let y_val = construct_wide_pointer(
            addr_of(x, <&usize>::get_type()),
            const_vtable(usize_a_vtable),
            ref_ty_default_markers_for(trait_obj_a_ty),
        );
        main.storage_live(y);
        main.assign(y, y_val);

        let foo_ret = main.declare_local::<usize>();
        main.storage_live(foo_ret);
        main.call(foo_ret, vtable_lookup(get_metadata(load(y)), method_a_foo), &[by_value(
            ptr_to_ptr(get_thin_pointer(load(y)), <&usize>::get_type()),
        )]);
        main.assume(eq(load(x), load(foo_ret)));

        main.exit();
        p.finish_function(main)
    };

    let p = p.finish_program(main);
    dump_program(p);
    assert_stop::<BasicMem>(p);
}

/// Tests that assigning a vtable defined for a different type, but same trait is fine.
///
/// This is not good code, but not defined as UB, not even in Miri.
///
/// ```rust
/// trait Foo {
///     fn foo(&self);
/// }
/// struct T1(u8);
/// struct T2(i8);
/// impl Foo for T1  {
///     fn foo(self: &T1) {
///         println!("{}", self.0)
///     }
/// }
/// impl Foo for T2 {
///     fn foo(self: &T2) {
///         println!("{}", self.0)
///     }
/// }
///
/// fn main() {
///     let x = T1(255);
///     let x_ptr = &x as *const T1 as *const T2;
///     // this will have a vtable for type T2, but a pointer to T1.
///     // According to the current reference, this should be UB.
///     let y: *const dyn Foo = x_ptr;
///
///     // this is definitely a problem now, it prints `-1`.
///     unsafe { &*y as &dyn Foo }.foo();
/// }
/// ```
#[test]
fn weird_wrong_vtable_right_trait() {
    let mut p = ProgramBuilder::new();

    // `trait Foo`
    let mut trait_foo = p.declare_trait();
    let method_foo_foo = trait_foo.declare_method();
    let trait_foo = p.finish_trait(trait_foo);

    // `impl Foo for u8`
    let impl_foo_foo_for_u8 = {
        let mut f = p.declare_function();
        let self_ = f.declare_arg::<&u8>();
        f.print(load(deref(load(self_), <u8>::get_type())));
        f.return_();
        p.finish_function(f)
    };
    let mut vtable_foo_u8 = p.declare_vtable_for_ty(trait_foo, <u8>::get_type());
    vtable_foo_u8.add_method(method_foo_foo, impl_foo_foo_for_u8);
    let _vtable_foo_u8 = p.finish_vtable(vtable_foo_u8);

    // `impl Foo for i8`
    let impl_foo_foo_for_i8 = {
        let mut f = p.declare_function();
        let self_ = f.declare_arg::<&i8>();
        f.print(load(deref(load(self_), <i8>::get_type())));
        f.return_();
        p.finish_function(f)
    };
    let mut vtable_foo_i8 = p.declare_vtable_for_ty(trait_foo, <i8>::get_type());
    vtable_foo_i8.add_method(method_foo_foo, impl_foo_foo_for_i8);
    let vtable_foo_i8 = p.finish_vtable(vtable_foo_i8);

    // `main()`
    let main = {
        let mut f = p.declare_function();
        // `let x = 255`
        let x = f.declare_local::<u8>();
        f.storage_live(x);
        f.assign(x, const_int(255_u8));

        // `let x_ptr: *const i8 = &raw x`
        let x_ptr = f.declare_local::<*const i8>();
        f.storage_live(x_ptr);
        f.assign(x_ptr, ptr_to_ptr(addr_of(x, <*const u8>::get_type()), <*const i8>::get_type()));

        // `let y: *const dyn Foo = x_ptr`
        let y = f.declare_local_with_ty(raw_ptr_ty(PointerMetaKind::VTablePointer(trait_foo)));
        f.storage_live(y);
        f.assign(
            y,
            construct_wide_pointer(
                load(x_ptr),
                // Statically the vtable for i8 would be used
                const_vtable(vtable_foo_i8),
                raw_ptr_ty(PointerMetaKind::VTablePointer(trait_foo)),
            ),
        );

        // `y.foo()`
        f.call_ignoreret(vtable_lookup(get_metadata(load(y)), method_foo_foo), &[by_value(
            ptr_to_ptr(get_thin_pointer(load(y)), <&u8>::get_type()),
        )]);

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main);
    assert_eq!(get_stdout::<BasicMem>(p).unwrap(), &["-1"]);
}

// UB tests

/// Makes sure this is UB:
/// ```rust
/// fn foo(x: *const dyn std::fmt::Debug) {}
///
/// fn main() {
///     foo(unsafe { std::mem::zeroed() });
/// }
/// ```
#[test]
fn ub_dangling_vtable() {
    let mut p = ProgramBuilder::new();

    let t_builder = p.declare_trait();
    let trait_name = p.finish_trait(t_builder);

    let foo = {
        let mut f = p.declare_function();
        f.declare_arg_with_ty(raw_ptr_ty(PointerMetaKind::VTablePointer(trait_name)));

        f.return_();
        p.finish_function(f)
    };

    let main = {
        let mut f = p.declare_function();
        let x = f.declare_local::<u8>();
        f.storage_live(x);

        let dangling_vtable_ptr = transmute(const_int(1_usize), Type::Ptr(PtrType::VTablePtr));
        let ptr_with_dangling_vtable = construct_wide_pointer(
            addr_of(x, <&u8>::get_type()),
            dangling_vtable_ptr,
            raw_ptr_ty(PointerMetaKind::VTablePointer(trait_name)),
        );

        f.call_ignoreret(fn_ptr(foo), &[by_value(ptr_with_dangling_vtable)]);
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main);
    assert_ub::<BasicMem>(p, "invalid pointer for vtable lookup");
}

/// The example, but instead tries to call a different method.
/// Might become ill-formed instead of UB when carrying the trait name with the PtrType::VTablePtr.
#[test]
fn ub_wrong_method() {
    let mut p = ProgramBuilder::new();

    let mut trait_a = p.declare_trait();
    let method_a_foo = trait_a.declare_method();
    let trait_a = p.finish_trait(trait_a);
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

        let trait_obj = main.declare_local_with_ty(ref_ty_default_markers_for(trait_obj_a_ty));
        let y_val = construct_wide_pointer(
            addr_of(x, <&usize>::get_type()),
            const_vtable(usize_a_vtable),
            ref_ty_default_markers_for(trait_obj_a_ty),
        );
        main.storage_live(trait_obj);
        main.assign(trait_obj, y_val);

        let foo_ret = main.declare_local::<usize>();
        main.storage_live(foo_ret);
        main.call(
            foo_ret,
            vtable_lookup(get_metadata(load(trait_obj)), TraitMethodName(Name::from_internal(42))),
            &[by_value(ptr_to_ptr(get_thin_pointer(load(trait_obj)), <&usize>::get_type()))],
        );
        main.assume(eq(load(x), load(foo_ret)));

        main.exit();
        p.finish_function(main)
    };

    let p = p.finish_program(main);
    assert_ub::<BasicMem>(p, "the referenced vtable does not have an entry for the invoked method");
}

/// It is UB for a wide pointer to point to a vtable for the wrong trait.
#[test]
fn ub_wrong_vtable_ty() {
    let mut p = ProgramBuilder::new();

    let t_builder = p.declare_trait();
    let trait1_name = p.finish_trait(t_builder);
    let v_builder = p.declare_vtable_for_ty(trait1_name, <u8>::get_type());
    let vtable1_name = p.finish_vtable(v_builder);
    let t_builder = p.declare_trait();
    let trait2_name = p.finish_trait(t_builder);

    let main = {
        let mut f = p.declare_function();
        let x = f.declare_local::<u8>();
        let y = f.declare_local_with_ty(raw_ptr_ty(PointerMetaKind::VTablePointer(trait2_name)));
        f.storage_live(x);
        f.storage_live(y);

        let wrong_trait_ptr = construct_wide_pointer(
            addr_of(x, <&u8>::get_type()),
            const_vtable(vtable1_name),
            raw_ptr_ty(PointerMetaKind::VTablePointer(trait2_name)),
        );
        f.assign(y, wrong_trait_ptr);

        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(main);
    assert_ub::<BasicMem>(p, "Value::Ptr: invalid vtable in metadata");
}

// Ill-formed tests

/// A vtable constant must have type `PtrType::VTablePtr`.
#[test]
fn ill_const_wrong_ty() {
    let mut p = ProgramBuilder::new();

    let t_builder = p.declare_trait();
    let trait_name = p.finish_trait(t_builder);
    let v_builder = p.declare_vtable(trait_name, Size::ZERO, Align::ONE);
    let vtable_name = p.finish_vtable(v_builder);
    let false_vtable_ptr_ty = raw_ptr_ty(PointerMetaKind::VTablePointer(trait_name));

    let f = {
        let mut f = p.declare_function();
        let y = f.declare_local_with_ty(false_vtable_ptr_ty);
        f.storage_live(y);
        f.assign(y, ValueExpr::Constant(Constant::VTablePointer(vtable_name), false_vtable_ptr_ty));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "Constant::VTablePointer: non vtable pointer type");
}

/// A vtable constant must point to a defined vtable.
#[test]
fn ill_const_undef_vtable() {
    let mut p = ProgramBuilder::new();

    let fake_vtable_name = VTableName(Name::from_internal(0));

    let f = {
        let mut f = p.declare_function();
        let y = f.declare_local_with_ty(Type::Ptr(PtrType::VTablePtr));
        f.storage_live(y);
        f.assign(y, const_vtable(fake_vtable_name));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(p, "Constant::VTablePointer: invalid vtable name");
}

/// A VTableLookup only works on `PtrType::VTablePtr` (in particular not wide pointers with a vtable metadata).
#[test]
fn ill_lookup_wrong_ty() {
    let mut p = ProgramBuilder::new();

    let void_fn = {
        let mut f = p.declare_function();
        f.return_();
        p.finish_function(f)
    };
    let mut t_builder = p.declare_trait();
    let meth1 = t_builder.declare_method();
    let trait_name = p.finish_trait(t_builder);
    let mut v_builder = p.declare_vtable(trait_name, Size::ZERO, Align::ONE);
    v_builder.add_method(meth1, void_fn);
    let _vtable_name = p.finish_vtable(v_builder);

    let wrong_vtable_ptr_ty = raw_ptr_ty(PointerMetaKind::VTablePointer(trait_name));

    let f = {
        let mut f = p.declare_function();
        let x = f.declare_local_with_ty(wrong_vtable_ptr_ty);
        let y = f.declare_local_with_ty(Type::Ptr(PtrType::FnPtr));
        f.storage_live(x);
        f.storage_live(y);
        // Fails, x isn't a VTablePtr
        f.assign(y, vtable_lookup(load(x), meth1));
        f.exit();
        p.finish_function(f)
    };

    let p = p.finish_program(f);
    assert_ill_formed::<BasicMem>(
        p,
        "UnOp::VTableMethodLookup: invalid operand: not a vtable pointer",
    );
}

/// A vtable must always be defined for a declared trait.
#[test]
fn ill_vtables_wrong_trait() {
    let mut p = ProgramBuilder::new();

    let f = {
        let mut f = p.declare_function();
        f.exit();
        p.finish_function(f)
    };

    let mut p = p.finish_program(f);
    // Insert vtable without a defined trait (the builder api disallows this)
    p.vtables.insert(VTableName(Name::from_internal(2)), VTable {
        trait_name: TraitName(Name::from_internal(0)),
        size: Size::ZERO,
        align: Align::ONE,
        methods: Map::new(),
    });

    assert_ill_formed::<BasicMem>(p, "Program: vtable for unknown trait");
}

/// A vtable's methods must match the declared method on the trait.
#[test]
fn ill_vtables_wrong_methods() {
    let mut p = ProgramBuilder::new();

    let mut t_builder = p.declare_trait();
    let _meth1 = t_builder.declare_method();
    let trait_name = p.finish_trait(t_builder);

    let f = {
        let mut f = p.declare_function();
        f.exit();
        p.finish_function(f)
    };

    let mut p = p.finish_program(f);

    // Insert vtable without the method (the builder api catches this)
    p.vtables.insert(VTableName(Name::from_internal(1)), VTable {
        trait_name: trait_name,
        size: Size::ZERO,
        align: Align::ONE,
        methods: Map::new(),
    });

    assert_ill_formed::<BasicMem>(p, "Program: vtable has not the right set of methods");
}
