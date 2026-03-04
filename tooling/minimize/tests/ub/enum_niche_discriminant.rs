#[allow(unused)]
enum Foo {
    Var1,       // variant 0, tag 2
    Var2(bool), // variant 1, untagged (valid values are 0=false and 1=true)
    Var3,       // variant 2, tag 4
}

fn main() {
    let x = 3u8; // this represents Var2 but encoded as a non-niched variant, which doesn't make sense
    let invalid: *const Foo = (&raw const x).cast();
    unsafe {
        let _is_niched = matches!(*invalid, Foo::Var2(_));
    }
}
