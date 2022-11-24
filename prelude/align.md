This is basically a copy of the `Align` type in the Rust compiler.
See [Align](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_target/abi/struct.Align.html).

`Align` is always a power of two.

For convenience, Align implements `Ord`.

```rust
/// `raw` stores the align in bytes.
pub struct Align { raw: Int }

impl Align {
    pub const ONE: Align = Align { raw: Int::ONE };

    /// align is rounded up to the next power of two.
    pub fn from_bytes(align: impl Into<Int>) -> Align {
        let align = align.into();
        let raw = align.next_power_of_two();

        Align { raw }
    }

    pub fn bytes(self) -> Int {
        self.raw
    }

    /// Computes the best alignment possible for the given offset
    /// (the largest power of two that the offset is a multiple of).
    /// For an offset of `0`, it returns None.
    pub fn max_for_offset(offset: Size) -> Option<Align> {
        offset.bytes().trailing_zeros()
            .map(|trailing| {
                let bytes = Int::from(2).pow(trailing);

                Align::from_bytes(bytes)
            })
    }

    /// Lower the alignment, if necessary, such that the given offset
    /// is aligned to it (the offset is a multiple of the alignment).
    pub fn restrict_for_offset(self, offset: Size) -> Align {
        Align::max_for_offset(offset)
            .map(|align| align.min(self))
            .unwrap_or(self)
    }
}
```
