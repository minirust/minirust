extern crate intrinsics;
use intrinsics::*;

static X: (u8, ()) = (0, ());

fn main() {
    // this failed in older versions of minimize, as the global was allocated with align 1,
    // but the deref wanted align=8 (it used the preferred align instead of the ABI align).
    print(X.0);
}
