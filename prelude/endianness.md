The endianness defines how integers and pointers are encoded and decoded.

```rust
pub enum Endianness {
    LittleEndian,
    BigEndian,
}

pub use Endianness::*;

impl Endianness {
    /// If `signed == Signed`, the data is interpreted as two's complement.
    pub fn decode(self, signed: Signedness, bytes: List<u8>) -> BigInt { todo!() }

    /// This can fail (return `None`) if the `int` does not fit into `size` bytes,
    /// or if it is negative and `signed == Unsigned`.
    pub fn encode(self, signed: Signedness, size: Size, int: BigInt) -> Option<List<u8>> { todo!() }
}
```
