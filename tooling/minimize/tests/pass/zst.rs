include!("../helper/transmute.rs");

fn do_things_with_zst<ZST: Copy>(z: ZST) {
    unsafe {
        // `ptr` will point to unallocated memory.
        let ptr: *mut ZST = transmute(8usize);

        // Still, you can read ...
        let _x = *ptr;

        // ... and write to `ptr`.
        *ptr = z;
    }
}

#[derive(Clone, Copy)]
struct A;

fn main() {
    do_things_with_zst::<A>(A);
    do_things_with_zst::<()>(());
    do_things_with_zst::<[u128; 0]>([]);
    do_things_with_zst::<(A, (A,()))>((A, (A,())));
}
