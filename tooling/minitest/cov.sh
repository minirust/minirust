#!/bin/bash

RUSTFLAGS="-C instrument-coverage" cargo test --tests
llvm-profdata merge -sparse $(find -name "*.profraw") -o out.profdata

llvm-cov show -Xdemangler=rustfilt \
    $( \
      for file in \
        $( \
          RUSTFLAGS="-C instrument-coverage" \
            cargo test --tests --no-run --message-format=json \
              | jq -r "select(.profile.test == true) | .filenames[]" \
              | grep -v dSYM - \
        ); \
      do \
        printf "%s %s " --object $file; \
      done \
    ) \
  --instr-profile=out.profdata \
  --use-color \
  --show-line-counts-or-regions \
  --ignore-filename-regex='/.cargo/registry' \
  --ignore-filename-regex='minitest' \
  --ignore-filename-regex='miniutil' \
  --ignore-filename-regex='libspecr' \
  --show-instantiations=false | less -R

rm out.profdata
(cd ..; rm -f $(find -name "*.profraw"))
