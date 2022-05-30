# MiniRust prelude

Across all files in this repository, we assume some definitions to always be in scope.

```rust
/// All operations are fallible, so they return `Result`.  If they fail, that
/// means the program caused UB or put the machine to a halt.
type Result<T=()> = std::result::Result<T, TerminationInfo>;

/// Basically copies of the `Size` and `Align` types in the Rust compiler.
/// See <https://doc.rust-lang.org/nightly/nightly-rustc/rustc_target/abi/struct.Size.html>
/// and <https://doc.rust-lang.org/nightly/nightly-rustc/rustc_target/abi/struct.Align.html>.
type Size;
type Align;

/// Whether an integer value is signed or unsigned.
enum Signedness {
    Unsigned,
    Signed,
}
pub use Signedness::*;

/// Whether a pointer/reference/allocation is mutable or immutable.
enum Mutability {
    Mutable,
    Immutable,
}
pub use Mutability::*;


/// The type of mathematical integers.
/// We assume all the usual arithmetic operations to be defined.
type BigInt;

impl BigInt {
    /// Returns the unique value that is equal to `self` modulo `2^size.bits()`.
    /// If `signed == Unsigned`, the result is in the interval `0..2^size.bits()`,
    /// else it is in the interval `-2^(size.bits()-1) .. 2^(size.bits()-1)`.
    ///
    /// `size` must not be zero.
    fn modulo(self, signed: Signedess, size: Size) -> BigInt;

    /// Tests whether an integer is in-bounds of a finite integer type.
    fn in_bounds(self, signed: Signedess, size: Size) -> BigInt {
        self == self.modulo(signed, size)
    }
}
```
