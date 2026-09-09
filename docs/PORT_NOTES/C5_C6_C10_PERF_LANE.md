# C5 / C6 / C10 performance lane — 2026-09-09

**Incomplete; not ready to merge.** The simulate-only path, allocation census,
Criterion benchmark, release regression script and strict merge-check script
are implemented. Final oracle verification and a complete performance baseline
are blocked by inaccessible shared game data and unavailable analysis tools.
C5 is not reduced to zero or to the deferred fighter effect vectors alone.

## Changes and boundaries

- `crates/melee-sim/src/frame.rs`: `tick_without_snapshot()` runs the same
  scheduler; existing `tick()` snapshots the completed frame with its original
  frame number. No comparator, schema, serializer or expected values changed.
  Overlap bodies use the existing two-fighter array shape. RNG writers retain
  callback values in a construction-time buffer sized to registrations + music;
  diagnostic strings are formatted only when requested, preserving their text.
- `crates/melee-sim/src/effects.rs`: reuse the flush request buffer, stream
  resolved requests without per-request collections, cache effect joint IDs,
  and retain the animation event buffer. Effect creation still allocates.
- `crates/melee-sim/tests/alloc_gate.rs`: thread-local counting global allocator,
  one warm-up tick, 599 measured ticks, and separate simulate-only / record-producing
  gate-tick totals. Counts include alloc, alloc_zeroed and realloc; deallocation
  is not counted. Construction, printing and trace comparison are outside the
  measured intervals. Direct stdout printing makes measured totals visible even
  without `--nocapture`. The nonzero budgets are explicitly provisional.
- `crates/melee-sim/benches/ticks.rs`, `crates/melee-sim/Cargo.toml`, `Cargo.lock`:
  Criterion 0.8.2 with default features disabled; separate import and 600-tick
  measurements. Per-iteration batching keeps scene construction/destruction
  outside ticking and bounds live scene memory. No rendering or comparisons.
- `tools/perf-gate.sh`, `tools/perf_report.py`, `docs/PERF.md`: release binary
  size, text, per-crate bloat, LLVM copies by compiling crate, benchmark results,
  configurable regression limits, dated machine-readable blocks. Missing tools,
  invalid evidence and failed measurements fail closed. Incomplete/regressed
  runs never ratchet the baseline upward. Analysis-tool output remains unverified
  on this machine because those executables could not be installed.
- `tools/merge-check.sh`, `CLAUDE.md`: ancestry and checked-out revision checks,
  strict build/gate/M4/M5/particle/clippy/fmt chain, failure-marker scanning,
  one result line per step and immediate termination on failure. Cargo validation
  uses `--locked` to prevent lockfile writes. Release perf
  remains separate from `cargo gate`.
- `tools/tests/test_perf_gate.py`, `tools/tests/test_merge_check.py`: seventeen
  tests for performance budgets and merge guards, including tracked/staged game
  data, protected lane changes, missing oracle data and the narrow override.
  Merge tests use real temporary Git repositories and stub only Cargo.
- Sixteen `M4_*.md` / `M5_*.md` files moved unchanged from
  `crates/melee-ft/src/fighter/` to `docs/PORT_NOTES/`. Verified byte-for-byte
  against the deleted tracked originals using read-only `git diff`.
  References updated in `docs/{FALCO_DATA,FALCON_DATA,MARTH_DATA,PARTICLES,
  PEACH_DATA,PUFF_DATA,YOSHI_DATA}.md`. README had no matching references.

Moved filenames: `M4_DASH.md`, `M4_FALCO.md`, `M4_FALCON.md`, `M4_FALLS.md`,
`M4_FOX.md`, `M4_JUMP.md`, `M4_LEDGE.md`, `M4_MARTH.md`, `M4_PEACH.md`,
`M4_PUFF.md`, `M4_SHIELD.md`, `M4_TURNRUN.md`, `M4_YOSHI.md`,
`M5_COMBAT2.md`, `M5_COMBAT3.md`, `M5_HIT.md`.

TRACKER.md was not edited, per the follow-up instruction; its old report
references remain. No fighter Rust, scenarios, ROMs, traces or decomp contents
were edited. No commits, git write commands, Dolphin runs or subagents.
The initial `third_party/melee-decomp` symlink/type-change diff was preserved.

## Allocation census and unresolved scope

| State of implementation | Simulate-only / 599 ticks | Record-producing tick / 599 ticks |
|---|---:|---:|
| Frame-loop changes, before effect-buffer changes; directly measured | 27,850 (46.494157/tick) | 102,724 (171.492487/tick) |
| With effect-buffer changes; simulate-only measured during allocation stack probe | 27,301 (45.577629/tick) | 102,175 **inferred, not directly remeasured** |

The measured earlier snapshot overhead was 74,874 allocations, or
124.998331/tick. The provisional final snapshot budget subtracts the measured
549-allocation effect-buffer saving from the earlier 102,724 total. Both final
assertions need a direct, uninstrumented rerun with real fixtures; the last
`alloc_gate` invocation skipped because `PlCo.dat` was inaccessible. These are
not claims of a passed final allocation acceptance gate.

The trace probe captured allocation stacks for ticks 1..9 and 599, with counting
disabled while collecting/printing stacks. The temporary probe source was
removed. Its log is `/tmp/battlefield-alloc-probe.log`. It found additional
allocation sources beyond the task's listed frame-loop sites. Scope expansion
was requested asynchronously; no answer arrived during this work, so no
supporting-crate refactor was performed.

Remaining sites (line numbers in this lane):

| Site | Remaining allocation |
|---|---|
| `crates/melee-ft/src/fighter/effects.rs:91`, `:100`, `:123`, `:182` | Motion-change partition/resolution, immediate drain, graphics queues; explicitly deferred C4/C5-ft. |
| `crates/melee-ft/src/anim/playback.rs:168`, `:216`, `:597` | Animation replacement/cloning and temporary joint collection for AObj traversal. |
| `crates/melee-ft/src/anim/attach.rs:64` | Selected animation tracks and FigaTree grouping on motion changes. |
| `crates/melee-ft/src/collision/ecb.rs:60` | Seven-point ECB Vec; observed even on the late idle tick. |
| `crates/melee-gr/src/last/animation.rs:150`, `:159`, `:173`, `:190` | Per-tick joint/matrix collections and animation event/result storage. |
| `crates/hsd-particle/src/system.rs:112`, `:162`, `:180`, `:302`, `:462` | Generator/particle storage, texture/bank copies at spawn, stable-sort scratch. |
| `crates/hsd-particle/src/rng_sites.rs:47` | Draw-log growth after the first warm-up tick (including sim-owned logs). |
| `crates/hsd-anim/src/jobj.rs:1423`, `:1470` | Animation event-buffer growth. |
| `crates/melee-sim/src/effects.rs:110`, `:267`, `:369` | Request-buffer first growth, on-demand effect model/animation import, instance growth; reuse removes repeated temporary allocations, not all creation costs. |
| `crates/melee-sim/src/trace.rs:26`, `crates/melee-diff/src/snapshot.rs:65`, `:90` | Opt-in record phase, path strings and BTreeMap entries, plus PrefixSink path construction. |

The cited `frame.rs:50` collection builds scheduler registrations at construction.
The original `frame.rs:593` Rc/RefCell collection was the `#[cfg(test)]`
scheduler-order test, not a per-tick RNG allocator. The runtime's Rc/RefCell
owners are constructed once. `frame/combat.rs`, `frame/falco_bones.rs`,
`frame/falcon_bones.rs` and `frame/peach_bones.rs` are all declared under
`#[cfg(test)]` in `frame.rs:859..868`; their oracle collections remain unchanged.

## Performance and tool environment

Historical main reference: stripped 3,866,720 bytes; text 3,604,480 bytes;
0.25 seconds CPU for the old load + compare gate.

The release perf script ran through reporting and exited **1 / INCOMPLETE**:

```text
[INCOMPLETE] perf-gate: 3866784 stripped bytes, 3604480 text bytes; timing unavailable
```

Stripped size increased 64 bytes (about 0.00166%); text is unchanged. Both size
metrics satisfy the default 5% tolerance. `docs/PERF.md` contains the first dated
INCOMPLETE block, not a timing/instantiation baseline. Raw evidence is under
`target/perf/20260909T230733Z-39876/`.

An earlier successful `cargo bench` run, before the effect-buffer changes and
concurrent with the merge-check, produced these **non-baseline** measurements:

```text
start_fd_fox/load       time:   [737.56 ms 887.31 ms 1.0846 s]
start_fd_fox/ticks_600  time:   [267.57 ms 431.71 ms 619.91 ms]
                        thrpt:  [967.89  elem/s 1.3898 Kelem/s 2.2424 Kelem/s]
```

Criterion mean point estimates were 887.3058585 ms load and 431.7071835 ms ticks.
These noisy timings demonstrate the benchmark executed; they do not measure the
final implementation on an idle machine. The final benchmark attempt failed:

```text
benchmark requires /Users/nikhilunni/Projects/melee-lanes/battlefield/harness/roms/files/PlCo.dat
error: bench failed, to rerun pass `-p melee-sim --bench ticks`
```

Versions:

```text
rustc 1.96.0 (ac68faa20 2026-05-25)
cargo 1.96.0 (30a34c682 2026-05-25)
clippy 0.1.96 (ac68faa20c 2026-05-25)
rustfmt 1.9.0-stable (ac68faa20c 2026-05-25)
Criterion 0.8.2 (cached dependency, not a standalone tool installation)
```

Neither cargo-bloat nor cargo-llvm-lines was installed. Both direct installation
and the script's individual `cargo install <tool> --locked` attempts failed:

```text
[6] Couldn't resolve host name (Could not resolve host: index.crates.io)
```

The default Cargo registry source directory is read-only in this sandbox.
A workspace-local `CARGO_HOME=$PWD/target/perf-cargo-home` reads the existing
index/cache and unpacks missing cached sources into the writable target tree.
The cached zerocopy/derive pair 0.8.48 was selected with:

```sh
CARGO_HOME="$PWD/target/perf-cargo-home" cargo update --offline -p zerocopy --precise 0.8.48
```

No global tool or package directory was modified. An ordinary unsandboxed Cargo
invocation can use the checked-in lockfile and fetch/unpack its dependencies.

## Commands and validation evidence

For post-Criterion Cargo commands below, the shell exported:

```sh
export CARGO_HOME="$PWD/target/perf-cargo-home"
```

| Exact command | Result / final lines |
|---|---|
| `cargo gate` before edits | Exit 0; aggregate **941 passed, 0 failed, 3 ignored**, including M4 261 and M5 8 with fixtures present. Log `/tmp/battlefield-initial-gate.log`. |
| `cargo install cargo-bloat cargo-llvm-lines --root target/perf-tools` | Exit 101; both installs failed on crates.io DNS. Log `/tmp/battlefield-perf-install.log`. |
| `cargo test -p melee-sim --test alloc_gate -- --nocapture` before Criterion/effect-buffer changes | Exit 0; `599 measured ticks: simulate-only 27850 allocations (46.494157/tick); with snapshot 102724 allocations (171.492487/tick); snapshot overhead 74874 (124.998331/tick)`. Initial assertion-free census. |
| `cargo bench --offline -p melee-sim --bench ticks --no-run` | Exit 0; `Finished bench profile [optimized + debuginfo] ... Executable benches/ticks.rs`. Log `/tmp/battlefield-bench-build.log`. |
| `cargo bench --offline -p melee-sim --bench ticks` | Exit 0 before shared assets broke; timing lines above, log `/tmp/battlefield-bench.log`. |
| `tools/merge-check.sh lane/battlefield` | Exit 1: `[PASS] rebase`, `[PASS] build`, `[FAIL] gate`; stopped at eight M4 fixture-read failures. Log `/tmp/battlefield-merge-check.log`. |
| `cargo test --offline -p melee-sim --test alloc_probe -- --nocapture` | Temporary stack probe: 27,301 simulate-only allocations; exit 0. Probe removed afterward. |
| `tools/perf-gate.sh` | Exit 1; first INCOMPLETE block written; final line quoted above. Log `/tmp/battlefield-perf-gate.log`. |
| `cargo clippy --offline --workspace --all-targets -- -D warnings` | Exit 0; `Finished dev profile [unoptimized + debuginfo] target(s) in 35.29s`. Log `/tmp/battlefield-clippy.log`. |
| `cargo fmt --all` | Exit 0, no output. Only edited/new sim Rust required formatting. |
| `cargo gate -- --nocapture` on final source | Exit 0; aggregate **942 passed, 0 failed, 3 ignored**, but **403 skip messages** because fixtures are inaccessible. Not oracle verification. Log `/tmp/battlefield-final-gate.log`. |
| `cargo test -p melee-sim --test m4_gate -- --nocapture` | Exit 0; `261 passed; 0 failed; 0 ignored`, **all 261 fixture cases skipped at runtime**. Log `/tmp/battlefield-final-m4.log`. |
| `cargo test -p melee-sim --test m5_gate -- --nocapture` | Exit 0; `8 passed; 0 failed; 0 ignored`, **all 8 fixture cases skipped at runtime**. Log `/tmp/battlefield-final-m5.log`. |
| `cargo test -p melee-sim --test alloc_gate -- --nocapture` on final source | Exit 0; `skipping allocation gate: .../PlCo.dat absent`; `1 passed; 0 failed; 0 ignored`. No final counts measured. Log `/tmp/battlefield-final-alloc.log`. |
| `cargo fmt --all -- --check` | Exit 0, no output. Log `/tmp/battlefield-final-fmt.log`. |
| `python3 -m unittest discover -s tools/tests -v` | Exit 0; `Ran 8 tests ... OK`. Log `/tmp/battlefield-script-tests.log`. |
| `bash -n tools/merge-check.sh` and `bash -n tools/perf-gate.sh` | Exit 0. |
| `git diff --check` | Exit 0; only the environment's fsmonitor socket diagnostic. |

## External blocker and next required verification

During validation, the **main checkout** asset directories changed into
self-referential symlinks (observed Sep 9 at 16:05 local):

```text
/Users/nikhilunni/Projects/melee/harness/roms -> /Users/nikhilunni/Projects/melee/harness/roms
/Users/nikhilunni/Projects/melee/harness/traces -> /Users/nikhilunni/Projects/melee/harness/traces
```

This lane's pre-existing links still target those paths. Eight M4 tests failed
mid-run with missing-file / `Too many levels of symbolic links (os error 62)`
errors; later runs skipped absent fixtures. No bit-exact mismatch was observed,
but final exactness is unverified. Those excluded paths were not repaired.

Restore the main asset directories, resolve the broader C5 scope, and make the
two analysis tools available. Then directly calibrate both allocation assertions,
finish removing in-scope allocations, rerun the unchanged oracle suites and
`tools/merge-check.sh lane/battlefield`, and run `tools/perf-gate.sh` on an idle
machine for a complete first baseline. Do not treat the skip-only runs or the
inferred snapshot budget as completed acceptance.

## Review follow-up: data guards

Updated files: `tools/merge-check.sh`, `tools/tests/test_perf_gate.py`, new
`tools/tests/test_merge_check.py`, and this report. No allocation changes or
TRACKER.md edits. The wider allocation census is deferred to **C5-b** after
local data is restored; it is no longer awaiting a scope ruling for this lane.

The merge script now checks data before ancestry/build:

1. `git ls-files -z -- harness/roms harness/traces` rejects any tracked entry,
   including directory symlinks and staged additions, with
   `[FAIL] data: <path> is tracked`.
2. `git diff --no-renames --name-only -z main...<branch> --` rejects exact protected paths or
   descendants under `harness/roms`, `harness/traces`, and
   `third_party/melee-decomp`, including a changed submodule pointer or deletion. Rename detection is disabled
   so moving a protected path elsewhere cannot hide its removal.
   Failure: `[FAIL] data: lane commits touch <path>`.
3. The trace directory is searched recursively while following symlinks for a
   regular `*.expected.jsonl` file. Missing, empty, broken and circular links
   fail. `--allow-missing-data`, before or after the branch argument, downgrades
   only this oracle-presence failure to `[WARN]`; it cannot bypass the tracked
   data or lane-diff protections.

Tests build minimal Git objects, refs and a v2 index directly in temporary
repositories, without any Git write commands. The script's Git reads are real;
only Cargo is stubbed to verify ordering and early termination without game data.
The fixture tests cover inherited and staged tracked data, symlinks, nested paths
and spaces, protected deletions and renames, a changed/unchanged decomp gitlink, similarly
named unprotected paths, oracle symlink traversal, missing/wrong-suffix/directory-only
oracle entries, and override scope. Existing strict-chain tests now use these
repository fixtures too.

Validation:

```sh
python3 -m unittest discover -s tools/tests -v
# Ran 17 tests ... OK
bash -n tools/merge-check.sh
# exit 0
tools/merge-check.sh lane/battlefield
# exit 1, before any Cargo command:
# [PASS] data: no tracked game data or protected lane changes
# [FAIL] data: harness/traces is empty; the gates would pass without running the oracle (use --allow-missing-data for a code-only lane)
git diff --check
# exit 0 (existing fsmonitor diagnostic only)
```

Logs: `/tmp/battlefield-data-guard-tests.log` and
`/tmp/battlefield-data-guard-live.log`. The live missing-oracle rejection is the
intended outcome for the currently inaccessible data, not a new test regression.
