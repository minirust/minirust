#!/bin/bash
exec cargo run --manifest-path=tooling/minimize/Cargo.toml -- "$@"
