# MiniRust prelude

Across all files in this repository, we assume some definitions to always be in scope.

```rust
/// All operations are fallible, so they return `Result`.  If they fail, that
/// means the program caused UB. What exactly the `UndefinedBehavior` type is
/// does not matter here.
type Result<T=()> = std::result::Result<T, UndefinedBehavior>;

/// Basically copies of the `Size` and `Align` types in the Rust compiler.
/// See <https://doc.rust-lang.org/nightly/nightly-rustc/rustc_target/abi/struct.Size.html>
/// and <https://doc.rust-lang.org/nightly/nightly-rustc/rustc_target/abi/struct.Align.html>.
type Size;
type Align;
```
