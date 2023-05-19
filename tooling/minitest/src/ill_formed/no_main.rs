use crate::*;

#[test]
fn no_main() {
    let p = program(&[]);
    assert_ill_formed(p);
}
