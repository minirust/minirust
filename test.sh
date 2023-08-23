#!/bin/bash
set -ex

# Fixed specr-transpile version
SPECR_VERSION="0.1.21"

# Stricter checks on CI
if [ -n "$CI" ]; then
    export RUSTFLAGS="-D warnings"
    export CARGOFLAGS="--locked"
fi

cargo install "specr-transpile@${SPECR_VERSION}"
specr-transpile specr.toml --check

cargo test --manifest-path=tooling/minitest/Cargo.toml $CARGOFLAGS
cargo test --manifest-path=tooling/minimize/Cargo.toml $CARGOFLAGS
