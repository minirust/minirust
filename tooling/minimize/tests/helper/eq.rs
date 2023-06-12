// funny little hack to do "is-zero"-checks even though minirust doesn't support any Eq, Lt, Gt etc. operators.
fn is_zero_u8(x: u8) -> bool {
    // equivalent to x%2
    fn mod2(x: u8) -> u8 {
        x - x/2*2
    }

    // these lines are equivalent to (x >> i) % 2.
    let bit0 = mod2(x);
    let bit1 = mod2(x/2);
    let bit2 = mod2(x/4);
    let bit3 = mod2(x/8);
    let bit4 = mod2(x/16);
    let bit5 = mod2(x/32);
    let bit6 = mod2(x/64);
    let bit7 = mod2(x/128);

    let out = bit0;
    let out = (out + bit1 + 1) / 2; // this is a logical or.
    let out = (out + bit2 + 1) / 2;
    let out = (out + bit3 + 1) / 2;
    let out = (out + bit4 + 1) / 2;
    let out = (out + bit5 + 1) / 2;
    let out = (out + bit6 + 1) / 2;
    let out = (out + bit7 + 1) / 2;

    let out = 1 - out; // this is a logical negation.

    #[repr(C)]
    union A {
        b: bool,
        u: u8,
    }
    let a = A { u: out };

    unsafe { a.b }
}
