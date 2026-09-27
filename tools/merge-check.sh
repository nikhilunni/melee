#!/usr/bin/env bash
# Validate the checked-out lane tree before Claude fast-forwards main.
set -euo pipefail
# A merge must exercise local oracles, regardless of the variable's value.
for variable in MELEE_ALLOW_MISSING_DATA MELEE_TEST_DATA_ROOT; do
    if [[ ${!variable+x} ]]; then
        echo "[FAIL] data: $variable is set; unset it before running the merge chain"
        exit 1
    fi
done
cd "$(dirname "$0")/.."
usage() {
    echo "usage: tools/merge-check.sh [--allow-missing-data] <lane-branch>" >&2
    exit 2
}
branch=
allow_missing_data=false
for argument in "$@"; do
    case "$argument" in
        --allow-missing-data) allow_missing_data=true ;;
        -*) usage ;;
        *)
            [[ -z "$branch" ]] || usage
            branch=$argument
            ;;
    esac
done
[[ -n "$branch" ]] || usage
logs=$(mktemp -d "${TMPDIR:-/tmp}/melee-merge-check.XXXXXX")
trap 'rm -rf "$logs"' EXIT

# Never let a lane replace local game-data directories with tracked entries,
# or change the shared decomp pointer. NUL delimiters preserve literal paths.
if ! git ls-files -z -- harness/roms harness/traces >"$logs/tracked"; then
    echo "[FAIL] data: cannot inspect tracked game data"
    exit 1
fi
while IFS= read -r -d '' path; do
    echo "[FAIL] data: $path is tracked"
    exit 1
done <"$logs/tracked"
# Disable rename detection so moving a protected path away still lists its deletion.
if ! git diff --no-renames --name-only -z "main...$branch" -- >"$logs/lane-paths"; then
    echo "[FAIL] data: cannot inspect lane commits"
    exit 1
fi
while IFS= read -r -d '' path; do
    case "$path" in
        harness/roms|harness/roms/*|harness/traces|harness/traces/*|third_party/melee-decomp|third_party/melee-decomp/*)
            echo "[FAIL] data: lane commits touch $path"
            exit 1
            ;;
    esac
done <"$logs/lane-paths"
echo "[PASS] data: no tracked game data or protected lane changes"

# Follow the lane's trace-directory symlink. A legacy command-line override
# cannot turn an empty oracle directory into a mergeable tree. Recorded traces
# may be zstd-compressed (<name>.expected.jsonl.zst, harness/trace_io.py).
oracle=$(find -L harness/traces -type f \( -name '*.expected.jsonl' -o -name '*.expected.jsonl.zst' \) -print -quit 2>/dev/null || true)
if [[ -z "$oracle" ]]; then
    echo "[FAIL] data: harness/traces is empty; record the required scenarios with harness/record.py <scenario>"
    if [[ "$allow_missing_data" == true ]]; then
        echo "[FAIL] data: --allow-missing-data no longer permits a code-only merge chain"
    fi
    exit 1
fi
echo "[PASS] data: oracle traces present"

# Gate the tree actually being built, including the lane's uncommitted edits.
if ! git merge-base --is-ancestor main "$branch" 2>"$logs/ancestry"; then
    echo "[FAIL] rebase: main is not an ancestor of $branch (or ref lookup failed)"
    cat "$logs/ancestry" >&2
    exit 1
fi
if [[ $(git log -1 --format=%H HEAD) != $(git log -1 --format=%H "$branch" --) ]]; then
    echo "[FAIL] rebase: checked-out HEAD is not $branch"
    exit 1
fi
if [[ -n $(git diff --name-only --diff-filter=U) ]]; then
    echo "[FAIL] rebase: unresolved merge conflicts"
    exit 1
fi
echo "[PASS] rebase: main is an ancestor of $branch; checking the lane tree"

step() {
    local name=$1 pattern=$2
    shift 2
    local log="$logs/$name"
    if "$@" >"$log" 2>&1; then
        if ! grep -Eq "$pattern" "$log"; then
            echo "[PASS] $name"
            return
        fi
    fi
    echo "[FAIL] $name"
    cat "$log" >&2
    exit 1
}
# --locked prevents validation from updating the source tree's Cargo.lock.
step build 'error\[|could not compile' cargo build --workspace --all-targets --locked
step gate 'error\[|could not compile|FAILED|panicked' cargo gate --locked
step m4_gate 'error\[|could not compile|FAILED|panicked' cargo test -p melee-sim --test m4_gate --locked
step m5_gate 'error\[|could not compile|FAILED|panicked' cargo test -p melee-sim --test m5_gate --locked
step hsd-particle 'error\[|could not compile|FAILED|panicked' cargo test -p hsd-particle --locked
step clippy 'error\[|could not compile|^error:' cargo clippy --workspace --all-targets --locked -- -D warnings
step fmt 'Diff in' cargo fmt --all -- --check
