const HELLO: &'static str = "Hello strings";

fn id(x: &'static str) -> &'static str {
    x
}

fn main() {
    assert!(id(HELLO) == "Hello strings");
}
