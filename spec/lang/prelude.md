# MiniRust Language prelude

For the files in this folder, we assume some definitions and parameters to always be in scope.

```rust
use crate::prelude::*;
use mem::{Memory, AbstractByte, Pointer, IntPtrCast};

// Everything there is to say about how an argument is passed to a function,
// and how the return value is passed back.
// For example, for stack passing this should say whether and how the
// stack is aligned before passing the argument and how many bytes of
// stack space to use; for register passing it should say which register
// to use and whether to do adjustments like sign extension.
// `Call` does not even check that caller and callee agree on the size
// (and indeed for register passing, mismatching size might be okay).
// FIXME: This is incomplete.
#[non_exhaustive]
pub enum ArgAbi {
    Register,
    Stack(Size, Align),
}
```
