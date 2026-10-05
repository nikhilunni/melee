#!/usr/bin/env bash
# Cross-target check: the pure math crates' tests under wasm32-wasip1 in
# wasmtime, then the same seeded matches native and under wasm, hashed per
# tick and diffed. The web build (wasm32-unknown-unknown) shares the code
# generation and differs from wasip1 only in libm, which gekko-math avoids.
#   tools/wasm-check.sh                 tests, then 3 matches x 2000 ticks
#   tools/wasm-check.sh --tests-only
#   WASM_CHECK_TICKS=500 tools/wasm-check.sh
# wasmtime is not installed by this script: put it on PATH or point
# MELEE_WASMTIME at the binary (a release from
# https://github.com/bytecodealliance/wasmtime/releases, unpacked anywhere).
set -euo pipefail
tests_only=0
case ${1:-} in
    --tests-only) tests_only=1 ;;
    '') ;;
    *) echo "usage: tools/wasm-check.sh [--tests-only]" >&2; exit 2 ;;
esac
cd "$(dirname "$0")/.."
root=$PWD

wasmtime=${MELEE_WASMTIME:-$(command -v wasmtime || true)}
if [[ -z $wasmtime || ! -x $wasmtime ]]; then
    cat >&2 <<'EOF'
wasmtime not found. Download a release for this host from
  https://github.com/bytecodealliance/wasmtime/releases
(e.g. wasmtime-v49.0.2-aarch64-macos.tar.xz), unpack it anywhere, and run
  MELEE_WASMTIME=/path/to/wasmtime tools/wasm-check.sh
EOF
    exit 1
fi
rustup target list --installed | grep -qx wasm32-wasip1 \
    || { echo 'missing target: rustup target add wasm32-wasip1' >&2; exit 1; }

# Game data: this checkout's, or MELEE_DATA_ROOT's (a worktree reads the
# main checkout in place) for the match hash. Crate tests read their own
# checkout's data by absolute path, so a worktree also needs
# MELEE_ALLOW_MISSING_DATA=1, passed through to the tests.
data_root=${MELEE_DATA_ROOT:-$root}
runner="$wasmtime run --dir $root::$root"
[[ $data_root != "$root" ]] && runner+=" --dir $data_root::$data_root"
[[ -n ${MELEE_ALLOW_MISSING_DATA:-} ]] && runner+=" --env MELEE_ALLOW_MISSING_DATA=$MELEE_ALLOW_MISSING_DATA"
export CARGO_TARGET_WASM32_WASIP1_RUNNER=$runner

# Pure crates whose suites run under WASI. Their native-C oracles skip
# themselves (no subprocesses); disc-data tests read through the preopen.
# melee-lb is out: its dynamics oracle needs a native subprocess. Release
# too: LLVM folds (the fnmadds negation, select-to-max) need optimization.
crates=(-p gekko-math -p hsd-types -p melee-types -p hsd-gobj -p hsd-anim -p melee-mp -p melee-cmd -p melee-coll)
echo "== cargo test (debug, wasm32-wasip1): ${crates[*]}"
cargo test -q --target wasm32-wasip1 "${crates[@]}"
echo "== cargo test (release, wasm32-wasip1): ${crates[*]}"
cargo test -q --release --target wasm32-wasip1 "${crates[@]}"
[[ $tests_only == 1 ]] && exit 0

files=$data_root/harness/roms/files
if [[ ! -f $files/PlCo.dat ]]; then
    echo "no extracted disc at $files; skipping the match hash (see docs/ISO.md)" >&2
    exit 1
fi
ticks=${WASM_CHECK_TICKS:-2000}
target_dir=${CARGO_TARGET_DIR:-$root/target}
out=$target_dir/wasm-check
mkdir -p "$out"
echo "== cross_target_hash: 3 matches x $ticks ticks, native vs wasm32-wasip1"
cargo build -q --release -p melee-lib --example cross_target_hash
cargo build -q --release -p melee-lib --example cross_target_hash --target wasm32-wasip1
"$target_dir/release/examples/cross_target_hash" "$files" "$ticks" >"$out/native.txt"
$wasmtime run --dir "$files::/files" \
    "$target_dir/wasm32-wasip1/release/examples/cross_target_hash.wasm" /files "$ticks" >"$out/wasm.txt"
if ! cmp -s "$out/native.txt" "$out/wasm.txt"; then
    echo "native and wasm differ; first lines:" >&2
    diff "$out/native.txt" "$out/wasm.txt" | head -20 >&2
    exit 1
fi
echo "identical: $(wc -l <"$out/native.txt") lines ($out)"
