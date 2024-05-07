static X: (i64, i64) = (2, 3);

static mut Y: i32 = 0;

struct Rec<'a> {
    this: &'a Rec<'a>,
    val: i32,
}

static RECURSIVE: Rec = Rec {
    this: &RECURSIVE,
    val: 42,
};

fn main() {
    let x = X;
    assert!(x.0 == 2);
    assert!(x.1 == 3);

    assert!(RECURSIVE.val == 42);
    assert!(RECURSIVE.this.val == 42);

    unsafe {
        assert!(Y == 0);
        Y = 1;
        assert!(Y == 1);
    }
}
