const HELLO: &'static str = "Hello strings";

fn id(x: &str) -> &str {
    x
}

fn main() {
    assert!(id(HELLO).len() == 13);

    let name = &id("Hej Björn!")[4..9];
    assert!(name.len() == 5);
    let mut chars = name.chars();
    assert!(chars.next() == Some('B'));
    assert!(chars.next() == Some('j'));
    assert!(chars.next() == Some('ö'));
    assert!(chars.next() == Some('r'));
    assert!(chars.next() == None);

    // TODO:
    // Comparing strings does not work yet in part due to lack of the `compare_bytes` intrinsic.
    // assert!(name == "Björn");

    // Various pattern based functions don't work because of lack of closures.
    // Various other things don't work because of missing MIR.
}
