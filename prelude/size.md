`Size` represents a non-negative number of bytes or bits.

It is basically a copy of the `Size` type in the Rust compiler.
See [Size](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_target/abi/struct.Size.html).

Note that the `Size` type has no upper-bound.
Users needs check whether a given `Size` is too large for their Machine themselves.

```rust
/// `raw` stores the size in bytes.
pub struct Size { raw: Int }

impl Size {
    pub const ZERO: Size = Size { raw: Int::ZERO };

    /// Rounds `bits` up to the next-higher byte boundary, if `bits` is
    /// not a multiple of 8.
    /// Will panic if `bits` is negative.
    pub fn from_bits(bits: impl Into<Int>) -> Size {
        let bits = bits.into();

        if bits < 0 {
            panic!("attempting to create negative Size");
        }

        let raw = bits.div_ceil(8);
        Size { raw }
    }

    /// variation of `from_bits` for const contexts.
    /// Cannot fail since the input is unsigned.
    pub const fn from_bits_const(bits: u64) -> Size {
        let bytes = bits.div_ceil(8);
        let raw = Int::from(bytes);
        Size { raw }
    }

    /// Will panic if `bytes` is negative.
    pub fn from_bytes(bytes: impl Into<Int>) -> Size {
        let bytes = bytes.into();

        if bytes < 0 {
            panic!("attempting to create negative Size");
        }

        Size { raw: bytes }
    }

    /// variation of `from_bytes` for const contexts.
    /// Cannot fail since the input is unsigned.
    pub const fn from_bytes_const(bytes: u64) -> Size {
        let raw = Int::from(bytes);
        Size { raw }
    }

    pub fn bytes(self) -> Int { self.raw }
    pub fn bits(self) -> Int { self.raw * 8 }
}
```
