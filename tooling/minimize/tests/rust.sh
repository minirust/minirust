#!/bin/bash

# This file is used run a test using `rustc` instead of `minimize`.

set -e

TEST_PATH="/tmp/minimize-testing"

[ ! -f "rust.sh" ] && echo 'You need to be in the `tests/` folder to execute `rust.sh`' && exit 1
[ -z $1 ] && echo "missing file to execute!" && exit 1

arg="$(readlink -f "$1")"
wd="$(pwd)"

(cd ../intrinsics; cargo b)

[ ! -d "$TEST_PATH" ] && mkdir "$TEST_PATH"
[ -f "$TEST_PATH/out" ] && rm "$TEST_PATH/out"
cp "$wd/../../rust-toolchain.toml" "$TEST_PATH"
cd "$TEST_PATH"

rustc "$arg" -o out -L "$wd/../intrinsics/target/debug" -l intrinsics -Zalways-encode-mir -Zmir-emit-retag -Zmir-opt-level=0 --cfg=miri -Zextra-const-ub-checks -Cdebug-assertions=off
./out
