#!/usr/bin/env bash
# CI-sized check: native math oracles in both profiles plus every rustc opt level.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test -p gekko-math
cargo test -p gekko-math --release
scratch=$(mktemp -d "${TMPDIR:-/tmp}/gekko-math-opts.XXXXXX")
trap 'rm -rf "$scratch"' EXIT
for opt in 0 1 2 3 s z; do
    echo "Fused math: opt-level=$opt"
    rustc --edition=2021 --test crates/gekko-math/src/fma.rs \
        -C "opt-level=$opt" -o "$scratch/fma-tests"
    "$scratch/fma-tests"
done
