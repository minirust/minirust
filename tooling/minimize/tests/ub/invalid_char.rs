#[allow(unnecessary_transmutes)]
fn main() {
    let surrogate = 0xD800u32;
    let _ub: char = unsafe { core::mem::transmute(surrogate) };
}
