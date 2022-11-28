This is basically a copy of the `Align` type in the Rust compiler.
See [Align](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_target/abi/struct.Align.html).

`Align` is always a power of two.

```rust,ignore
pub use specr::Align;

impl Align {
    pub const ONE: Align;

    /// align is rounded up to the next power of two.
    pub fn from_bytes(align: impl Into<Int>) -> Align;

    /// Returns the align in bytes.
    pub fn bytes(self) -> Int;

    /// Computes the best alignment possible for the given offset
    /// (the largest power of two that the offset is a multiple of).
    /// For an offset of `0`, it returns None.
    pub fn max_for_offset(offset: Size) -> Option<Align>;

    /// Lower the alignment, if necessary, such that the given offset
    /// is aligned to it (the offset is a multiple of the alignment).
    pub fn restrict_for_offset(self, offset: Size) -> Align;
}
```
