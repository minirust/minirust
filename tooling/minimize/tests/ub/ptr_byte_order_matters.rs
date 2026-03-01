//! Ensure that swapping two bytes in a pointer invalidates the provenance.
use std::mem::{self, MaybeUninit};

unsafe fn swap<T: Copy>(x: *mut T, y: *mut T) {
    let tmp = *x;
    *x = *y;
    *y = tmp;
}

fn main() {
    // We construct a pointer where swapping the two least significant bytes keeps it definitely
    // inbounds. This operation can move the pointer by at most 0xFFFF, so if we make the allocation
    // sufficiently big, a pointer in the middle will have this property.
    let buf = [0u8; 2*0xFFFF + 2];
    let mid_ptr: *const u8 = buf.as_ptr().wrapping_add(buf.len() / 2);
    let mut ptr_bytes: [MaybeUninit<u8>; 8] = unsafe { mem::transmute(mid_ptr) };
    unsafe { swap(&raw mut ptr_bytes[0], &raw mut ptr_bytes[1]) }; // little-endian: swap the first two bytes
    let swapped_ptr: *const u8 = unsafe { mem::transmute(ptr_bytes) };
    let _val = unsafe { *swapped_ptr };
}
