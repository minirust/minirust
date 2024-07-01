fn main() {
    let x = [1, 2];
    let i = black_box(2);
    // FIXME: use `get_unchecked` (after unsized types are implemented)
    // here to create out-of-bounds UB and make this test fail instead.
    let _y = x[i];
}

fn black_box(i: usize) -> usize { i }
