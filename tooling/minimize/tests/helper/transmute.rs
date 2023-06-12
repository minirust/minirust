#[allow(unused)]
union Transmute<T: Copy, U: Copy> {
    t: T,
    u: U,
}

#[allow(unused)]
unsafe fn transmute<T: Copy, U: Copy>(t: T) -> U {
    unsafe {
        Transmute { t }.u
    }
}
