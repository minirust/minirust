This folder contains various tools built on top of MiniRust:

- `minituil`: general utilities for interacting with MiniRust programs from Rust code, mainly to more
  easily construct MiniRust programs and to debug-print constructed MiniRust programs.
- `minitest`: test suite of MiniRust programs.
- `minimize`: generates MiniRust from Rust (via MIR). Also helps test MiniRust, by having test cases
  written in Rust and executed as MiniRust programs.

`minimize` directly links against rustc, so you need a nightly toolchain installed to build it. The
`rust-toolchain.toml` file in the repository root lists the required nightly version and extra
component, so generally this should all be installed automatically. To work with `minimize` in
vscode, add the following to your workspace settings:
```
"rust-analyzer.rustc.source": "discover",
```
