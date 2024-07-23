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

        let x = &mut foo.bar as *mut u8; 
        let y = &mut *x; 
        let yraw = y as *mut u8;

        *x.add(1) = 42; 

        assert!(*yraw.add(0) == 42);

        *yraw.add(1) = 57;
    }
}
