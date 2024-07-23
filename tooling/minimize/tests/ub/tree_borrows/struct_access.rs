#[allow(dead_code)]
struct Foo {
    bar: u8, 
    baz: u8,
}


//@ compile-flags: --minimize-tree-borrows
fn main() {
    unsafe {
        let mut foo = Foo {
            bar: 42,
            baz: 57
        }; 

        let x = &mut foo.bar as *mut u8; // (x, [R, R])
        let y = &mut *x; // (x, [R, R]) -> (y, [R, R])
        let yraw = y as *mut u8; // (x, [R, R]) -> (y, [R, R])

        // Using x at offset 1 invalidates y.
        *x.add(1) = 42; // (x, [R, A]) -> (y, [R, D])

        // y can still be used for all the other offsets
        assert!(*yraw.add(0) == 42); // (x, [R, A]) -> (y, [R, D])

        // UB! The write to x has invalidated its child, y.
        *yraw.add(1) = 57;
    }
}
