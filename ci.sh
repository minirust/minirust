#!/bin/bash
set -ex

# Fixed `minirust-tooling` commit, we need to bump this occasionally.
TOOLING_COMMIT="8831033"

# where to check out the tooling
TOOLING_DIR="$HOME/minirust-tooling"

git clone "https://github.com/memoryleak47/minirust-tooling" "$TOOLING_DIR"
ln -s "$PWD" "$TOOLING_DIR"/minirust

cd "$TOOLING_DIR"
git checkout "$TOOLING_COMMIT"

# transpile, and build the transpiled result
(cd specr-transpile; cargo run)
(cd gen-minirust; RUSTFLAGS="-D warnings" cargo build)
