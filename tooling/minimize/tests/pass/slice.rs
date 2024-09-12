/// This should act pretty much exactly like implicit unsizing, including lifetimes.
fn unsize<'a, T, const N: usize>(arr: &'a [T; N]) -> &'a [T] {
    #[repr(C)]
    struct SlicePointerTuple<T> {
        begin: *const T,
        len: usize,
    }

    // We do not have normal unsizing yet, so we cheat.
    // FIXME(UnsizedTypes): Don't cheat, or at least use `as_ptr()`, which requires unsizing tho.
    let ptr = SlicePointerTuple { begin: arr as *const T, len: N };
    // SAFETY: SlicePointerTuple has the same layout as `&[T]` and since arr is only accessible behind
    // a reference, no mutable reference can exist
    let slice = unsafe { core::mem::transmute::<SlicePointerTuple<T>, &[T]>(ptr) };
    slice
}

// Pass slice reference accross function boundaries
pub fn assert_some_elements(a: &[i32]) {
    assert!(a[1] == -40);
    assert!(a[2] == 30);
    assert!(a[3] == -20);
}

fn main() {
    let x: [i32; 5] = [50, -40, 30, -20, 10];
    let slice = unsize(&x);

    assert_some_elements(slice);
    assert!(slice.len() == 5);
}
