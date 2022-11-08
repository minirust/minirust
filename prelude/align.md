This is basically a copy of the `Align` type in the Rust compiler.
See [Align](https://doc.rust-lang.org/nightly/nightly-rustc/rustc_target/abi/struct.Align.html).

`Align` is always a power of two.

```rust
/// `raw` stores the align in bytes.
pub struct Align { raw: BigInt }

impl Align {
    /// align is rounded up to the next power of two.
    pub fn from_bytes(align: impl Into<BigInt>) -> Align {
        let align = align.into();
        let raw = align.next_power_of_two();

        Align { raw }
    }

    pub fn bytes(self) -> BigInt {
        self.raw
    }
}
```
