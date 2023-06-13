#!/bin/bash

RUSTFLAGS="-C instrument-coverage" cargo t
llvm-profdata merge -sparse $(find -name "*.profraw") -o out.profdata

llvm-cov show -Xdemangler=rustfilt target/debug/minimize \
  --instr-profile=out.profdata \
  --use-color \
  --show-line-counts-or-regions \
  --ignore-filename-regex='/.cargo/registry' \
  --ignore-filename-regex='miniutil' \
  --ignore-filename-regex='libspecr' \
  --show-instantiations=false | less -R

rm out.profdata
rm -f $(find -name "*.profraw")
