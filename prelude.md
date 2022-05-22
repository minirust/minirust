# MiniRust prelude

Across all files in this repository, we assume some definitions to always be in scope.

```rust
/// Basically copies of the `Size` and `Align` types in the Rust compiler.
/// See <https://doc.rust-lang.org/nightly/nightly-rustc/rustc_target/abi/struct.Size.html>
/// and <https://doc.rust-lang.org/nightly/nightly-rustc/rustc_target/abi/struct.Align.html>.
type Size;
type Align;
```
