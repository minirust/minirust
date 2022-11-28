Int is the type of mathematical integers.

We assume all the usual arithmetic operations to be defined.
Additionally, Int provides a few utility functions.

```rust,ignore
pub use specr::Int;

impl Int {
    pub const ZERO: Int;
    pub const ONE: Int;

    /// Converts any integer type to Int.
    pub fn from(x: impl Into<Int>) -> Int;

    /// Returns the next-higher power of two.
    pub fn next_power_of_two(&self) -> Int;

    /// Checks whether `self` is a power of two.
    pub fn is_power_of_two(&self) -> bool;

    /// Returns the absolute value of `self`.
    pub fn abs(self) -> Int;

    /// Returns `self` to the power of `exp`.
    pub fn pow(self, exp: impl Into<Int>) -> Int;

    /// Computes `self / other`, returns None if `other` is zero.
    pub fn checked_div(other: impl Into<Int>) -> Option<Int>;

    /// Returns the unique value that is equal to `self` modulo `2^size.bits()`.
    /// If `signed == Unsigned`, the result is in the interval `0..2^size.bits()`,
    /// else it is in the interval `-2^(size.bits()-1) .. 2^(size.bits()-1)`.
    ///
    /// `size` must not be zero.
    pub fn modulo(self, signed: Signedness, size: Size) -> Int;

    /// Tests whether an integer is in-bounds of a finite integer type.
    pub fn in_bounds(self, signed: Signedness, size: Size) -> bool;

    /// Rounded up division.
    pub fn div_ceil(other: impl Into<Int>) -> Int;

    /// Returns the number of least-significant bits that are zero, or None if the entire number is zero.
    pub fn trailing_zeros(self) -> Option<Int>;

}
```
