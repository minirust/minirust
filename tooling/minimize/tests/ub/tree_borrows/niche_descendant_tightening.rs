//@ compile-flags: --minirust-tree-borrows

// §4.11 of the proposal: a partial Foreign Read of the niche region fires
// the niche check, whose phantom Local Reads through `T_n` propagate to
// `T_c` (descendant) as Foreign Reads -- tightening `T_c` `Unique → Frozen`
// at the un-touched niche bytes. A subsequent Local Write through `T_c` at
// a tightened byte then hits Frozen and UBs.

static V: u32 = 0x11223344;
static W: u32 = 0xAABBCCDD;

fn main() {
    let mut x: Option<&'static u32> = Some(&V);
    let p = match &mut x {
        Some(r) => r as *mut &'static u32,
        None => unreachable!(),
    };
    unsafe {
        // Step 1: Local Write through T_c -- T_c becomes Unique at the pointer's bytes.
        *p = &W;
        // Step 2: Partial Foreign Read of byte 0 through a reborrow of x.
        // The niche check fires (touches byte 0). Phantom Local Reads through
        // T_n cover the other 7 pointer bytes; these propagate to T_c as
        // Foreign Reads, transitioning T_c at bytes 1..8 from Unique to Frozen.
        let _b = *(&x as *const Option<&'static u32> as *const u8);
        // Step 3: Local Write through T_c at byte 1 -- T_c at byte 1 is now
        // Frozen → UB.
        *(p as *mut u8).wrapping_add(1) = 0xFE;
    }
}
