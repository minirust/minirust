# MiniRust prelude

Across all files in this repository, we assume some definitions to always be in scope.

```rust
/// All operations are fallible, so they return `Result`.  If they fail, that
/// means the program caused UB or put the machine to a halt.
type Result<T=()> = std::result::Result<T, TerminationInfo>;

#[non_exhaustive]
enum TerminationInfo {
  Ub(String),
  MachineStop(String),
}

/// Some macros for convenient yeeting (yes this is valid syntax on nightly Rust).
macro_rules! throw {
    ($($tt:tt)*) => { do yeet None };
}
macro_rules! throw_ub {
    ($($tt:tt)*) => { do yeet TerminationInfo::Ub(format!($($tt)*)) };
}
macro_rules! throw_machine_stop {
    ($($tt:tt)*) => { do yeet TerminationInfo::MachineStop(format!($($tt)*)) };
}

/// We leave the encoding of the non-determinism monad opaque.
type Nondet<T=()>;
type NdResult<T=()> = Nondet<Result<T>>;

/// Basically copies of the `Size` and `Align` types in the Rust compiler.
/// See <https://doc.rust-lang.org/nightly/nightly-rustc/rustc_target/abi/struct.Size.html>
/// and <https://doc.rust-lang.org/nightly/nightly-rustc/rustc_target/abi/struct.Align.html>.
///
/// `Size` is essentially a `BigInt` newtype that is always in-bounds for both
/// signed and unsigned `Memory::PTR_SIZE` (i.e., it is in the range `0..=isize::MAX`).
/// `Size::from_bytes` and the checked arithmetic operations return `None`
/// when the result would be out-of-bounds.
/// `Align` is additionally always a power of two.
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

/// The endianness, which defines how integers and pointers are encoded and decoded.
enum Endianness {
    LittleEndian,
    BigEndian,
}

impl Endianness {
    /// If `signed == Signed`, the data is interpreted as two's complement.
    fn decode(self, signed: Signedness, bytes: List<u8>) -> BigInt;

    /// This can fail (return `None`) if the `int` does not fit into `size` bytes,
    /// or if it is negative and `signed == Unsigned`.
    fn encode(self, signed: Signedness, size: Size, int: BigInt) -> Option<List<u8>>;
}

/// The type of mathematical integers.
/// We assume all the usual arithmetic operations to be defined.
type BigInt;

impl BigInt {
    /// Returns the unique value that is equal to `self` modulo `2^size.bits()`.
    /// If `signed == Unsigned`, the result is in the interval `0..2^size.bits()`,
    /// else it is in the interval `-2^(size.bits()-1) .. 2^(size.bits()-1)`.
    ///
    /// `size` must not be zero.
    fn modulo(self, signed: Signedness, size: Size) -> BigInt;

    /// Tests whether an integer is in-bounds of a finite integer type.
    fn in_bounds(self, signed: Signedness, size: Size) -> bool {
        self == self.modulo(signed, size)
    }
}
```
