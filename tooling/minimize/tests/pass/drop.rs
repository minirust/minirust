extern crate intrinsics;
use intrinsics::*;

struct Bomb;

impl Drop for Bomb {
    fn drop(&mut self) {
        print(42);
    }
}

trait Shell {}
impl Shell for Bomb {}

fn main() {
    // Drop once normally
    let _b = Bomb;

    // Then drop as trait object for Shell.
    let mut bomb = Bomb;
    // drop at dyn type
    unsafe {
        std::ptr::drop_in_place(&mut bomb as &mut dyn Shell);
    }
    // prevent double-drop
    std::mem::forget(bomb);
}
