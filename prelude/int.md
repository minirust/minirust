Int is the type of mathematical integers.

We assume all the usual arithmetic operations to be defined.
Additionally, Int provides a few utility functions.

```rust
pub use specr::Int;

impl Int {
    pub const ZERO: Int = Int::from(0);
    pub const ONE: Int = Int::from(1);

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
    pub fn modulo(self, signed: Signedness, size: Size) -> Int {
        if size.is_zero() {
            panic!("Int::modulo received invalid size zero!");
        }

        // the modulus.
        let m = Int::from(2).pow(size.bits());

        // n is in range `-(m-1)..m`.
        let n = self % m;

        match signed {
            // if `Unsigned`, output needs to be in range `0..m`:
            Unsigned if n < 0 => n + m,
            // if `Signed`, output needs to be in range `-m/2 .. m/2`:
            Signed if n >= m/2 => n - m,
            Signed if n < -m/2 => n + m,
            _ => n,
        }
    }

    /// Tests whether an integer is in-bounds of a finite integer type.
    pub fn in_bounds(self, signed: Signedness, size: Size) -> bool {
        self == self.modulo(signed, size)
    }

    /// Rounded up division.
    pub fn div_ceil(other: impl Into<Int>) -> Int;

    /// Returns the number of least-significant bits that are zero, or None if the entire number is zero.
    pub fn trailing_zeros(self) -> Option<Int>;

}
```
