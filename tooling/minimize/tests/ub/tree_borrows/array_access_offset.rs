//@ compile-flags: --minimize-tree-borrows
fn main() {
    unsafe {
        let mut data = [0u8, 1, 2, 3];
        let x = &mut data[0] as *mut u8; // (x, [R, R, R, R])
        let y = &mut *x; // (x, [R, R, R, R]) -> (y, [R, R, R, R])
        let yraw = y as *mut u8;
        // Using x at offset 1 invalidates y.
        *x.add(1) = 42; // (x, [R, A, R, R]) -> (y, [R, D, R, R])
        // y can still be used for all the other offsets
        *yraw.add(0) = 42;
        *yraw.add(2) = 42;
        *yraw.add(3) = 42;

        assert!(*yraw.add(0) == 42);
        assert!(*yraw.add(2) == 42);
        assert!(*yraw.add(3) == 42);

        // UB! The write to x has invalidated its child, y.
        *yraw.add(1) = 57;
    }
}
