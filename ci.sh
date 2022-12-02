#!/bin/sh

# Fixed `minirust-tooling` commit, we need to bump this occasionally.
TOOLING_COMMIT="0441a98"

git clone "https://github.com/memoryleak47/minirust-tooling" ~/minirust-tooling
ln -s ~/work/minirust/minirust ~/minirust-tooling/minirust

cd ~/minirust-tooling
git checkout "$TOOLING_COMMIT"

(cd specr-transpile; cargo run)
(cd gen-minirust; cargo build)
