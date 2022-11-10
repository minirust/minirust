This is basically a copy of the `Size` type in the Rust compiler.
See [Size](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_target/abi/struct.Size.html).

Note that the `Size` type has no upper-bound.
Users needs check whether a given `Size` is too large for their Machine themselves.

```rust
/// `raw` stores the size in bytes.
pub struct Size { raw: BigInt }

impl Size {
    /// Rounds `bits` up to the next-higher byte boundary, if `bits` is
    /// not a multiple of 8.
    pub fn from_bits(bits: impl Into<BigInt>) -> Size {
        let bits = bits.into();

        // round up `bits / 8`
        let raw = bits / 8 + ((bits % 8) + 7) / 8;
        Size { raw }
    }

    pub fn from_bytes(bytes: impl Into<BigInt>) -> Size {
        let bytes = bytes.into();
        Size { raw: bytes }
    }

    pub fn bytes(self) -> BigInt { self.raw }
    pub fn bits(self) -> BigInt { self.raw * 8 }
    pub fn is_zero(&self) -> bool { self.raw == 0 }
}

// We implement a few operators for size.
use std::ops::*;
use std::cmp::Ordering;

impl Add for Size {
    type Output = Size;

    fn add(self, rhs: Size) -> Size {
        let raw = self.raw + rhs.raw;
        Size { raw }
    }
}

impl Mul<BigInt> for Size {
    type Output = Size;

    fn mul(self, rhs: BigInt) -> Size {
        let raw = self.raw * rhs;
        Size { raw }
    }
}

impl PartialEq for Size {
    fn eq(&self, rhs: &Size) -> bool {
        self.raw == rhs.raw
    }
}

impl PartialOrd for Size {
    fn partial_cmp(&self, rhs: &Size) -> Option<Ordering> {
        self.raw.partial_cmp(&rhs.raw)
    }
}

impl Ord for Size {}
```
