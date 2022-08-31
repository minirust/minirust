# MiniRust Language prelude

For the files in this folder, we assume some definitions and parameters to always be in scope.

```rust
// An instance of the memory interface.
use crate::mem::interface::*;
type Memory: MemoryInterface;

type Provenance = Memory::Provenance;
type Pointer = Memory::Pointer;
type AbstractByte = Memory::AbstractByte;

// The endianess, which defines how integers are encoded and decoded.
trait Endianess {
    /// If `signed == Signed`, the data is interpreted as two's complement.
    fn decode<const N: usize>(self, signed: Signedness, bytes: [u8; N]) -> BigInt;

    /// This can fail (return `None`) if the `int` does not fit into `N` bytes,
    /// or if it is negative and `signed == Unsigned`.
    fn encode<const N: usize>(self, signed: Signedness, int: BigInt) -> Option<[u8; N]>;
}
const ENDIANESS: impl Endianess;

// Everything there is to say about how an argument is passed to a function,
// and how the return value is passed back.
// For example, for stack passing this should say whether and how the
// stack is aligned before passing the argument; for register passing
// it should say which register to use and whether to do adjustments
// like sign extension.
// The only thing that the general `Call` implementation ensures is that
// caller and callee agree on the size of the argument/return value.
type ArgAbi: PartialEq;
```
