//@ compile-flags: --minimize-tree-borrows

// Check that a foreign read from a location only freezes
// the active mutable reference at that offset.
// After that, writing to this location via this mutable reference is UB.
// Other locations are not affected.

fn main() {
    unsafe {
        let mut data = [0u8, 1, 2, 3];
        let x = &mut data[0] as *mut u8; // (x, [R, R, R, R])
        let y = &mut *x; // (x, [R, R, R, R]) -> (y, [R, R, R, R])
        let yraw = y as *mut u8;
        // Using y at offset 1 activate x and y.
        *yraw.add(1) = 42; // (x, [R, A, R, R]) -> (y, [R, A, R, R])
        // Using x at offset 1 freezes y.
        assert!(*x.add(1) == 42); // (x, [R, A, R, R]) -> (y, [R, F, R, R])
        // y can still be used for all the other offsets
        *yraw.add(0) = 42;
        *yraw.add(2) = 42;
        *yraw.add(3) = 42;

        assert!(*yraw.add(0) == 42);
        assert!(*yraw.add(2) == 42);
        assert!(*yraw.add(3) == 42);

        // UB! The read from x has frozen its child, y.
        *yraw.add(1) = 57;
    }
}
