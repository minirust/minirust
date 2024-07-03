use crate::*;

#[test]
fn inbounds_success() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let arr = f.declare_local::<[u32; 4]>();
    let ptr1 = f.declare_local::<*const i32>();
    let ptr2 = f.declare_local::<*const i32>();
    f.storage_live(arr);
    f.storage_live(ptr1);
    f.storage_live(ptr2);
    f.assign(ptr1, addr_of(index(arr, const_int(0i32)), <*const i32>::get_type()));
    f.assign(ptr2, addr_of(index(arr, const_int(1i32)), <*const i32>::get_type()));
    f.assume(eq(ptr_offset_from(load(ptr2), load(ptr1), InBounds::Yes), const_int(4isize)));
    f.assume(eq(ptr_offset_from(load(ptr2), load(ptr1), InBounds::No), const_int(4isize)));
    f.assume(eq(ptr_offset_from(load(ptr1), load(ptr2), InBounds::Yes), const_int(-4isize)));
    f.assume(eq(ptr_offset_from(load(ptr1), load(ptr2), InBounds::No), const_int(-4isize)));
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop(p);
}

#[test]
fn oob_success() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let var1 = f.declare_local::<u32>();
    let var1_addr = addr_of(var1, <*const u32>::get_type());
    let var2 = f.declare_local::<u32>();
    let var2_addr = addr_of(var2, <*const u32>::get_type());
    let diff = f.declare_local::<isize>();
    f.storage_live(var1);
    f.storage_live(var2);
    f.storage_live(diff);
    f.assign(diff, ptr_offset_from(var1_addr, var2_addr, InBounds::No));
    f.assume(eq(ptr_offset_from(var2_addr, var1_addr, InBounds::No), neg(load(diff))));
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop(p);
}

#[test]
fn inbounds_cross_alloc() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let var1 = f.declare_local::<u32>();
    let var1_addr = addr_of(var1, <*const u32>::get_type());
    let var2 = f.declare_local::<u32>();
    let var2_addr = addr_of(var2, <*const u32>::get_type());
    let diff = f.declare_local::<isize>();
    f.storage_live(var1);
    f.storage_live(var2);
    f.storage_live(diff);
    f.assign(diff, ptr_offset_from(var1_addr, var2_addr, InBounds::Yes));
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_ub(p, "dereferencing pointer outside the bounds of its allocation");
}

#[test]
fn inbounds_cross_alloc_same_addr() {
    let mut p = ProgramBuilder::new();

    let mut f = p.declare_function();
    let var1 = f.declare_local::<u32>();
    let var1_addr = addr_of(var1, <*const u32>::get_type());
    let var2 = f.declare_local::<u32>();
    let var2_addr = addr_of(var2, <*const u32>::get_type());
    let diff = f.declare_local::<isize>();
    f.storage_live(var1);
    f.storage_live(var2);
    f.storage_live(diff);
    f.assign(diff, ptr_offset_from(var1_addr, var2_addr, InBounds::No));
    let var2_addr_offset = ptr_offset(var2_addr, load(diff), InBounds::No); // var2_addr + (var1_addr - var2_addr)
    f.assume(eq(ptr_offset_from(var1_addr, var2_addr_offset, InBounds::Yes), const_int(0isize)));
    f.exit();
    let f = p.finish_function(f);

    let p = p.finish_program(f);
    assert_stop(p);
}
