`Size` represents a non-negative number of bytes or bits.

It is basically a copy of the `Size` type in the Rust compiler.
See [Size](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_target/abi/struct.Size.html).

Note that the `Size` type has no upper-bound.
Users needs check whether a given `Size` is too large for their Machine themselves.

```rust
/// `raw` stores the size in bytes.
pub struct Size { raw: BigInt }

impl Size {
    pub const ZERO: Size = Size { raw: BigInt::ZERO };

    /// Rounds `bits` up to the next-higher byte boundary, if `bits` is
    /// not a multiple of 8.
    /// Will panic if `bits` is negative.
    pub fn from_bits(bits: impl Into<BigInt>) -> Size {
        let bits = bits.into();

        if bits < 0 {
            panic!("attempting to create negative Size");
        }

        // round up `bits / 8`
        let raw = bits / 8 + ((bits % 8) + 7) / 8;
        Size { raw }
    }

    /// Will panic if `bytes` is negative.
    pub fn from_bytes(bytes: impl Into<BigInt>) -> Size {
        let bytes = bytes.into();

        if bytes < 0 {
            panic!("attempting to create negative Size");
        }

        Size { raw: bytes }
    }

    pub fn bytes(self) -> BigInt { self.raw }
    pub fn bits(self) -> BigInt { self.raw * 8 }
}
```
