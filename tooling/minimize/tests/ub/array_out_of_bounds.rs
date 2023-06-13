fn main() {
    let x = [1, 2];
    let i = black_box(2);
    let _y = x[i];
}

fn black_box(i: usize) -> usize { i }
