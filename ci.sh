#!/bin/bash
set -ex

# Fixed specr-transpile version
VERSION="0.1.18"

cargo install "specr-transpile@${VERSION}"
specr-transpile specr.toml

(cd tooling/minirust-rs; RUSTFLAGS="-D warnings" cargo build)
(cd tooling/minitest; cargo test)
(cd tooling/minimize; cargo test)
