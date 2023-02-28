#!/bin/bash
set -ex

# Fixed specr-transpile version
VERSION="0.1.4"

cargo install "specr-transpile@${VERSION}"
specr-transpile specr.toml

cd gen-minirust
RUSTFLAGS="-D warnings" cargo build
