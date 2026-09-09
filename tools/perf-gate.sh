#!/usr/bin/env bash
# Release-only regression gate. No oracle comparisons and no rendering.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $# != 0 ]]; then
    echo 'usage: tools/perf-gate.sh (PERF_TIME_TOLERANCE=10 PERF_SIZE_TOLERANCE=5 PERF_COPIES_TOLERANCE=0)' >&2
    exit 2
fi
export CARGO_TERM_COLOR=never
root=$PWD
target_dir=$(cargo metadata --no-deps --format-version=1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')
run_dir="$target_dir/perf/$(date -u +%Y%m%dT%H%M%SZ)-$$"
mkdir -p "$run_dir"
export PATH="$target_dir/perf-tools/bin:$PATH"
# Install into build artifacts, not the user's global Cargo bin directory.
for tool in cargo-bloat cargo-llvm-lines; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Installing missing $tool into $target_dir/perf-tools"
        if ! cargo install "$tool" --locked --root "$target_dir/perf-tools" >"$run_dir/install-$tool.txt" 2>&1; then
            echo "Could not install $tool; this run will be incomplete and fail." >&2
            cat "$run_dir/install-$tool.txt" >&2
        fi
    fi
done
rustc --version >"$run_dir/rustc.txt"
cargo --version >"$run_dir/cargo.txt"
git log -1 --format=%H >"$run_dir/revision.txt"

echo 'Building release melee-sim'
cargo build --release -p melee-sim --bin melee-sim --message-format=json >"$run_dir/build.jsonl"
python3 - "$run_dir" <<'PY'
import json, pathlib, shutil, sys
run = pathlib.Path(sys.argv[1])
artifacts = [json.loads(line) for line in (run / 'build.jsonl').read_text().splitlines()]
executables = [a['executable'] for a in artifacts if a.get('reason') == 'compiler-artifact'
               and a['target']['name'] == 'melee-sim' and a.get('executable')]
if len(executables) != 1:
    raise SystemExit('expected one melee-sim executable from cargo build')
shutil.copy2(executables[0], run / 'melee-sim.stripped')
PY
strip "$run_dir/melee-sim.stripped"
size "$run_dir/melee-sim.stripped" >"$run_dir/size.txt"

if command -v cargo-bloat >/dev/null 2>&1; then
    cargo bloat --version >"$run_dir/bloat-version.txt"
    if ! cargo bloat --release -p melee-sim --bin melee-sim --crates -n 0 >"$run_dir/bloat.txt" 2>"$run_dir/bloat-error.txt"; then
        touch "$run_dir/bloat.failed"
    fi
fi
if command -v cargo-llvm-lines >/dev/null 2>&1; then
    cargo llvm-lines --version >"$run_dir/llvm-version.txt"
    # Monomorphizations are charged to their compiling crate. Inspect the sim
    # library (which owns the generic scene dispatch), plus each character crate.
    cargo metadata --no-deps --format-version=1 | python3 -c 'import json,sys; print("\n".join(sorted(p["name"] for p in json.load(sys.stdin)["packages"] if p["name"].startswith("ft-"))))' >"$run_dir/characters.txt"
    for package in melee-sim $(cat "$run_dir/characters.txt"); do
        if ! cargo llvm-lines -p "$package" --release --lib >"$run_dir/llvm-$package.txt" 2>"$run_dir/llvm-$package-error.txt"; then
            touch "$run_dir/llvm-$package.failed"
        fi
    done
fi
# A fresh output directory prevents an earlier Criterion run becoming evidence
# when a benchmark is skipped, fails, or changes its name.
export CRITERION_HOME="$run_dir/criterion"
echo 'Benchmarking load and 600 simulate-only ticks'
if ! cargo bench -p melee-sim --bench ticks -- --noplot 2>&1 | tee "$run_dir/bench.txt"; then
    touch "$run_dir/bench.failed"
fi
python3 "$root/tools/perf_report.py" "$run_dir" "$root/docs/PERF.md"
