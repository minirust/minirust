# MiniRust Language prelude

For the files in this folder, we assume some definitions and parameters to always be in scope.

```rust
// An instance of the memory interface.
use mem::interface::*;
type Memory: MemoryInterface;
use Memory::{Provenance, Pointer, AbstractByte};

// The size of a pointer.
const PTR_SIZE: Size;

// The endianess, which defines how integers are encoded and decoded.
trait Endianess {
    fn decode<N: usize>(self, signed: Signedness, bytes: [u8; N]) -> BigInt;
    /// This can fail if the `int` does not fit into `N` bytes, or if it is
    /// negative and `signed == Unsigned`.
    fn encode<N: usize>(self, signed: Signedness, int: BigInt) -> Option<[u8; N]>;
}
const ENDIANESS: impl Endianess;
```
