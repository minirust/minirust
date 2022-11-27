`Size` represents a non-negative number of bytes or bits.

It is basically a copy of the `Size` type in the Rust compiler.
See [Size](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_target/abi/struct.Size.html).

Note that the `Size` type has no upper-bound.
Users needs check whether a given `Size` is too large for their Machine themselves.

For convenience, we assume that `Size + Size` and `Size * Int` are implemented implicitly.

```rust,ignore
pub use specr::Size;

impl Size {
    pub const ZERO: Size;

    /// Rounds `bits` up to the next-higher byte boundary, if `bits` is
    /// not a multiple of 8.
    /// Will panic if `bits` is negative.
    pub fn from_bits(bits: impl Into<Int>) -> Size;

    /// variation of `from_bits` for const contexts.
    /// Cannot fail since the input is unsigned.
    pub const fn from_bits_const(bits: u64) -> Size;

    /// Will panic if `bytes` is negative.
    pub fn from_bytes(bytes: impl Into<Int>) -> Size;

    /// variation of `from_bytes` for const contexts.
    /// Cannot fail since the input is unsigned.
    pub const fn from_bytes_const(bytes: u64) -> Size;

    /// Returns the size in bytes.
    pub fn bytes(self) -> Int;

    /// Returns the size in bits.
    pub fn bits(self) -> Int;
}
```
