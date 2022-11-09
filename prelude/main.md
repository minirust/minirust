# MiniRust prelude

Across all files in this repository, we assume some definitions to always be in scope.

```rust
/// All operations are fallible, so they return `Result`.  If they fail, that
/// means the program caused UB or put the machine to a halt.
pub type Result<T=()> = std::result::Result<T, TerminationInfo>;

#[non_exhaustive]
pub enum TerminationInfo {
  Ub(String),
  MachineStop(String),
}

/// Some macros for convenient yeeting, i.e., return an error from a
/// `Option`/`Result`-returning function.
macro_rules! throw {
    ($($tt:tt)*) => {
        specr::yeet!(());
    };
}
macro_rules! throw_ub {
    ($($tt:tt)*) => {
        specr::yeet!(TerminationInfo::Ub(format!($($tt)*)));
    };
}
macro_rules! throw_machine_stop {
    ($($tt:tt)*) => {
        specr::yeet!(TerminationInfo::MachineStop(format!($($tt)*)));
    };
}

/// We leave the encoding of the non-determinism monad opaque.
pub use specr::Nondet;
pub type NdResult<T=()> = Nondet<Result<T>>;

/// Whether an integer value is signed or unsigned.
pub enum Signedness {
    Unsigned,
    Signed,
}
pub use Signedness::*;

/// Whether a pointer/reference/allocation is mutable or immutable.
pub enum Mutability {
    Mutable,
    Immutable,
}
pub use Mutability::*;
```
