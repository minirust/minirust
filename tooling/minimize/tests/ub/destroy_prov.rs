include!("../helper/transmute.rs");

fn main() { unsafe {
    let b = 2;
    let x = &b as *const i32; // valid ptr!

    // transmute round-trip
    let x = transmute::<*const i32, usize>(x);
    let x = transmute::<usize, *const i32>(x);

    let _x = *x; // invalid ptr!
} }
