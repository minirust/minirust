const HELLO: &'static str = "Hello strings";

fn id(x: &str) -> &str {
    x
}

fn main() {
    assert!(id(HELLO).len() == 13);

    let name = &"Hej Björn!"[4..9];
    assert!(name.len() == 5);

    // TODO:
    // Comparing strings does not work yet in part due to lack of the `compare_bytes` intrinsic.
    // assert!(name == "Björn");

    // Indexing does not work due to lack of chars.
    // Various pattern based functions don't work because of lack of closures.
    // Various other things don't work because of missing MIR.
}
