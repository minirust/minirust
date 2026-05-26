//@ compile-flags: --minirust-tree-borrows

// §4.3 of the proposal: after corrupting the niche bytes, reading *any* of
// the niche bytes through a tag foreign to `T_n` fires the niche check --
// the rest of the niche region is reconstructed via phantom Local Reads
// through `T_n` and the full decode fails, so UB fires at the byte read.

static V: u32 = 0xAABBCCDD;

fn main() {
    let mut x: Option<&'static u32> = Some(&V);
    let inner = match &mut x {
        Some(p) => p,
        None => unreachable!(),
    };
    // Null out the pointer through a raw cast (Local for T_n, no check).
    unsafe {
        *(inner as *mut &'static u32 as *mut usize) = 0;
    }
    // Foreign Read of a single niche byte through a reborrow of x. The
    // niche check phantom-reads the rest of the pointer bytes and decodes
    // them as a single usize == 0 → `None`, which doesn't match the
    // claimed `Some` → UB.
    let q = &x as *const Option<&'static u32> as *const u8;
    let _b = unsafe { *q };
}
