# C11: no silent oracle passes

Implemented on 2026-09-09 in the chars lane; no commits or Git write commands. Validation results below distinguish missing local evidence from a bit-exact divergence.

## Outcome and remaining blocker

- M4: **261 passed**, M5: **8 passed**, hsd-particle: **75 passed**.
- Strict full workspace audit: **951 passed, 3 failed, 3 pre-existing ignored**. All failures are missing data: JIT probes (`gekko-math/golden`), post-render Fox matrices (`melee-ft/start_fox_bones_130`), and legacy M2 bones (`melee-sim/m2_gate`). No numerical divergence occurred. The ordinary `cargo test -p melee-ft` stopped at that missing VI capture (88 passed, 1 failed before stopping); the full audit exercised the remaining targets.
- Explicit opt-out against the empty preflight root: **954 passed, 0 failed, 3 pre-existing ignored**. This proves the opt-out path works across the workspace; it is not evidence that those data oracles ran.
- Clippy, formatting, both helper-policy tests, and all 13 merge-check regression tests pass. The focused strict/opt-out M4 demonstration exits 101/0; merge-check rejects the permissive environment with exit 1.
- M4 timing: baseline 152.23s, changed 152.39s (0.11% difference); M5: 17.87s then 8.40s. Oracle loops and tick code are unchanged. `cargo tree -p melee-sim -e normal` contains no `melee-test-support` dependency.

Acceptance requiring a fully green strict workspace and fighter suite is **blocked by the absent captures**, not waived. The first strict gate found the interpreter probe missing; those interpreter captures appeared externally in the shared directory before the full audit. JIT probes, legacy M2 bones/metadata and both post-render VI bones/metadata remained absent at final inspection. Recording/restoration was not attempted because this lane must not write protected data paths.

The full numerical audit ran before the final recovery-hint wording refinement; the final clippy, policy tests, strict gate, missing-capture tests, and whole-workspace opt-out run use the final helper. Final diagnostics include executable scenario TOML arguments and dedicated bone-capture settings; earlier transcripts are retained as historical evidence.

## Shared policy

`melee-test-support::require_files` is a dependency-free dev-only helper. It checks the supplied files in order (including `Scenario::required_files()` plus each test’s additional ledgers/bones), panics at the first absent/non-file path, and names its recovery command. Only the exact environment value `MELEE_ALLOW_MISSING_DATA=1` prints the same diagnostic and returns `false`, letting the caller return early. That explicit opt-out is still reported as `ok` by libtest and is not oracle evidence. Existing expected values, comparison loops, thresholds, and allocation budgets are unchanged.

`MELEE_TEST_DATA_ROOT` replaces `harness/` for preflight checks only, enabling absence simulation against an empty scratch directory. It does not redirect scenario loading or production reads. Normal successful checks do not write output or allocate recovery messages. The helper is used only in tests (including `#[cfg(test)]` simulation modules); no gameplay dependency or tick-path work is added.

`tools/merge-check.sh` rejects either variable whenever it is set, including empty and `0` values, before running Git or Cargo. Empty trace directories now fail even with the legacy `--allow-missing-data` argument. The shell regression tests keep the tracked-data, protected-path, ancestry, ordering, and failure-marker checks.

Recovery messages use `docs/DISC.md` for roms and `harness/record.py <scenario>` for scene captures. Special capture formats have their actual recovery instructions: `harness/gekko_probe/run.sh 0` / `GEKKO_PROBE_OUT=harness/traces/jit harness/gekko_probe/run.sh 4` for probes, and `docs/M2_GATE.md` with the required environment settings for legacy `fox_ys` and post-render VI bone oracles. Ordinary hints include the scenario TOML path and applicable ledger/bone flags.

## Converted data preflights

Each row lists every helper call in a converted file; shared support files cover their callers’ suites. Directory-only guards now list actual DAT files. The probe replay requires all interpreter/JIT captures, including previously silent partial omissions. The fighter raw-bone alignment check also requires its capture instead of silently omitting that comparison.

| File | Helper call lines |
|---|---|
| `crates/ft-captain/tests/attributes.rs` | 67 |
| `crates/ft-falco/tests/attributes.rs` | 60 |
| `crates/ft-fox/tests/attributes.rs` | 201 |
| `crates/ft-mars/tests/attributes.rs` | 34 |
| `crates/ft-peach/tests/attributes.rs` | 96 |
| `crates/ft-purin/tests/attributes.rs` | 39 |
| `crates/ft-yoshi/tests/attributes.rs` | 77 |
| `crates/gekko-math/tests/golden.rs` | 160 |
| `crates/hsd-anim/tests/real_fox.rs` | 17 |
| `crates/hsd-archive/tests/real_dat.rs` | 24 |
| `crates/hsd-particle/tests/live_fd.rs` | 38 |
| `crates/hsd-particle/tests/live_fd_start.rs` | 32 |
| `crates/hsd-particle/tests/real_fd_bank.rs` | 10 |
| `crates/hsd-particle/tests/real_fd_particles.rs` | 35 |
| `crates/hsd-particle/tests/support/dust_replay.rs` | 122 |
| `crates/melee-ft/tests/fighter_support/mod.rs` | 74 |
| `crates/melee-ft/tests/fighter_support/rendered_pose.rs` | 18 |
| `crates/melee-ft/tests/fighter_support/replay.rs` | 68, 89, 138 |
| `crates/melee-ft/tests/human_idle_input_600.rs` | 16 |
| `crates/melee-ft/tests/idle_fox_600.rs` | 18 |
| `crates/melee-ft/tests/idle_ground_fields_600.rs` | 112 |
| `crates/melee-ft/tests/movement_fox_states.rs` | 41, 152 |
| `crates/melee-ft/tests/real_fox_data.rs` | 13 |
| `crates/melee-ft/tests/real_fox_wait_playback.rs` | 54, 61 |
| `crates/melee-ft/tests/start_fox_states.rs` | 19 |
| `crates/melee-gr/tests/real_battlefield.rs` | 8 |
| `crates/melee-gr/tests/real_fd.rs` | 12 |
| `crates/melee-gr/tests/real_pupupu.rs` | 8 |
| `crates/melee-gr/tests/real_story.rs` | 8 |
| `crates/melee-lb/tests/real_fox_wait.rs` | 18 |
| `crates/melee-mp/tests/real_stage.rs` | 20 |
| `crates/melee-sim/src/countdown.rs` | 57 |
| `crates/melee-sim/src/frame.rs` | 870 |
| `crates/melee-sim/src/frame/combat.rs` | 143 |
| `crates/melee-sim/src/frame/falco_bones.rs` | 57 |
| `crates/melee-sim/src/frame/falcon_bones.rs` | 57, 118 |
| `crates/melee-sim/src/frame/fall_states.rs` | 55 |
| `crates/melee-sim/src/frame/marth_bones.rs` | 57 |
| `crates/melee-sim/src/frame/peach_bones.rs` | 57 |
| `crates/melee-sim/src/frame/puff_bones.rs` | 57 |
| `crates/melee-sim/src/frame/puff_state.rs` | 10 |
| `crates/melee-sim/src/frame/yoshi_bones.rs` | 57 |
| `crates/melee-sim/src/frame/yoshi_state.rs` | 11 |
| `crates/melee-sim/src/initial_state/cold_tests.rs` | 96, 170, 194 |
| `crates/melee-sim/tests/alloc_gate.rs` | 68 |
| `crates/melee-sim/tests/fixture_spawns.rs` | 72 |
| `crates/melee-sim/tests/m2_gate.rs` | 39 |
| `crates/melee-sim/tests/m3_gate.rs` | 7 |
| `crates/melee-sim/tests/m4_gate.rs` | 7, 57 |
| `crates/melee-sim/tests/m5_gate.rs` | 52, 99 |
| `crates/melee-sim/tests/real_fox_bones.rs` | 10 |
| `crates/melee-sim/tests/slippi_oracle.rs` | 42, 84 |
| `crates/melee-sim/tests/slippi_replay.rs` | 66 |

## Non-data omissions retained

Compiler absence and unavailable decomp provenance remain permitted under their original conditions and now emit `[NON-DATA OMITTED]`. Native compilation/comparison errors still fail. The previously silent optional Wait IASA source-copy comparison now emits this marker too.

| File | Distinct diagnostic sites |
|---|---|
| `crates/gekko-math/tests/ref_oracle.rs` | 72, 93 |
| `crates/hsd-anim/tests/anim_ref_oracle.rs` | 40, 58 |
| `crates/hsd-anim/tests/mtx_oracle.rs` | 68, 84, 1049 |
| `crates/melee-ft/tests/airborne_ref_oracle.rs` | 30 |
| `crates/melee-ft/tests/animation_ref_oracle.rs` | 45 |
| `crates/melee-ft/tests/grounded_physics_native.rs` | 25 |
| `crates/melee-ft/tests/input_oracle.rs` | 16, 222 |
| `crates/melee-lb/tests/dynamics_ref_oracle.rs` | 14 |
| `crates/melee-lb/tests/ref_oracle.rs` | 64, 85 |
| `crates/melee-mp/tests/geom_oracle.rs` | 61, 83 |

These cover Gekko reference-copy/compiler checks; HSD animation and matrix copy/compiler/header checks; melee-lb trig copy/compiler and dynamics compiler checks; melee-mp geometry copy/compiler checks; and melee-ft airborne, animation, grounded-physics, controller, and Wait IASA provenance checks.

The two pre-existing `#[ignore]` raw combat-scratch regressions in `melee-sim/src/frame/combat.rs` remain unchanged (grab/throw and tech hitbox-phase evidence outside the 49-key gate). The pre-existing schema example doc-test at `melee-sim/src/schema.rs:15` also remains ignored. No new ignored tests were added.

`grep -rn 'skipping' crates/*/tests crates/*/src` now finds only two unrelated gameplay doc comments: `crates/melee-mp/src/query.rs:567` and `crates/melee-ft/src/fighter/commands.rs:201`. Neither is an early-return site; the latter is inside the explicitly forbidden `melee-ft/src` scope. The animation test name now says `crossing_segments`; its assertions are unchanged.

## Validation

All commands below ran from `/Users/nikhilunni/Projects/melee-lanes/chars`. Outputs are final result lines except where a diagnostic is necessary. Full temporary logs are `/tmp/c11-<label>.log`. The baseline `cargo gate` exited 0; it reported M4 261/261 in 152.23s and M5 8/8 in 17.87s, but also silently passed missing captures.

`cargo clippy --workspace --all-targets -- -D warnings` exited 0:
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 26.46s
```

`cargo fmt --all` exited 0 (no output).

`python3 -m unittest discover -s tools/tests -p test_merge_check.py` exited 0:
```text
.............
----------------------------------------------------------------------
Ran 13 tests in 2.218s

OK
```

Absence setup (no output):
```sh
mkdir -p /tmp/melee-c11-empty
```
The directory remained empty throughout; no real data directory was moved or modified.

### gate

```sh
cargo gate
```
Exit 101; wall time 31.9s.

Sum of reported test results: 42 passed, 1 failed, 0 ignored.

```text
missing oracle data: /Users/nikhilunni/Projects/melee-lanes/chars/crates/gekko-math/../../harness/traces/jit/frsqrte_probe.jsonl; restore with `GEKKO_PROBE_OUT=harness/traces/jit harness/gekko_probe/run.sh 4`; only contributors without local data may opt out with MELEE_ALLOW_MISSING_DATA=1

failures:
    full_captures_replay_bit_for_bit

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

error: test failed, to rerun pass `-p gekko-math --test golden`
```

### merge-env

```sh
MELEE_ALLOW_MISSING_DATA=1 tools/merge-check.sh lane/chars
```
Exit 1; wall time 0.01s.

```text
[FAIL] data: MELEE_ALLOW_MISSING_DATA is set; unset it before running the merge chain
```

### native-absent

```sh
CC=/tmp/melee-c11-no-compiler cargo test -p hsd-anim --test anim_ref_oracle helmite_matches_native_c_bit_for_bit -- --exact --nocapture
```
Exit 0; wall time 0.06s.

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.01s
     Running tests/anim_ref_oracle.rs (target/debug/deps/anim_ref_oracle-5616c2d1a015e9b9)

running 1 test
[NON-DATA OMITTED] no working C compiler (`/tmp/melee-c11-no-compiler`) on PATH; omitting native oracle comparison
test helmite_matches_native_c_bit_for_bit ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
```

### m4

```sh
cargo test -p melee-sim --test m4_gate
```
Exit 0; wall time 152.51s.

Sum of reported test results: 261 passed, 0 failed, 0 ignored.

```text
test wavedash_fd_yoshi_300 ... ok
test wavedash_marth_particle_draw_order ... ok
test wavedash_peach_particle_draw_order ... ok
test wavedash_puff_particle_draw_order ... ok
test wavedash_yoshi_particle_draw_order ... ok

test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 152.39s
```

### m5

```sh
cargo test -p melee-sim --test m5_gate
```
Exit 0; wall time 8.51s.

Sum of reported test results: 8 passed, 0 failed, 0 ignored.

```text
test tech_fd_marth_300_ticks_and_ordered_particle_draws ... ok
test shieldhit_fd_marth_300_ticks_and_ordered_particle_draws ... ok
test jab_fd_marth_300_ticks_and_ordered_particle_draws ... ok
test grab_fd_marth_300_ticks_and_ordered_particle_draws ... ok
test ko_fd_marth_480_ticks_and_ordered_particle_draws ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.40s
```

### particle

```sh
cargo test -p hsd-particle
```
Exit 0; wall time 58.87s.

Sum of reported test results: 75 passed, 0 failed, 0 ignored.

```text
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests hsd_particle

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### ft

```sh
cargo test -p melee-ft
```
Exit 101; wall time 27.62s.

Sum of reported test results: 88 passed, 1 failed, 0 ignored.

```text
missing oracle data: /Users/nikhilunni/Projects/melee-lanes/chars/crates/melee-ft/../../harness/traces/start_fd_fox.bones_vi_p0.jsonl; restore with `harness/record.py start_fd_fox`; only contributors without local data may opt out with MELEE_ALLOW_MISSING_DATA=1

failures:
    start_fox_matrices_vi_130

test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.27s

error: test failed, to rerun pass `-p melee-ft --test start_fox_bones_130`
```

### gate-all

```sh
cargo gate --no-fail-fast
```
Exit 101; wall time 504.91s.

Sum of reported test results: 951 passed, 3 failed, 3 ignored.

```text
missing oracle data: /Users/nikhilunni/Projects/melee-lanes/chars/crates/gekko-math/../../harness/traces/jit/frsqrte_probe.jsonl; restore with `GEKKO_PROBE_OUT=harness/traces/jit harness/gekko_probe/run.sh 4`; only contributors without local data may opt out with MELEE_ALLOW_MISSING_DATA=1
missing oracle data: /Users/nikhilunni/Projects/melee-lanes/chars/crates/melee-ft/../../harness/traces/start_fd_fox.bones_vi_p0.jsonl; restore with `harness/record.py start_fd_fox`; only contributors without local data may opt out with MELEE_ALLOW_MISSING_DATA=1
missing oracle data: /Users/nikhilunni/Projects/melee-lanes/chars/crates/melee-sim/../../harness/traces/fox_ys.bones.expected.jsonl; restore with the bone capture command in docs/M2_GATE.md (scenario idle_ys_fox); only contributors without local data may opt out with MELEE_ALLOW_MISSING_DATA=1

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: 3 targets failed:
    `-p gekko-math --test golden`
    `-p melee-ft --test start_fox_bones_130`
    `-p melee-sim --test m2_gate`
```

### clippy

```sh
cargo clippy --workspace --all-targets -- -D warnings
```
Exit 0; wall time 26.58s.

```text
    Checking ft-mars v0.0.1 (/Users/nikhilunni/Projects/melee-lanes/chars/crates/ft-mars)
    Checking melee-gr v0.0.1 (/Users/nikhilunni/Projects/melee-lanes/chars/crates/melee-gr)
    Checking ft-peach v0.0.1 (/Users/nikhilunni/Projects/melee-lanes/chars/crates/ft-peach)
    Checking ft-falco v0.0.1 (/Users/nikhilunni/Projects/melee-lanes/chars/crates/ft-falco)
    Checking ft-fox v0.0.1 (/Users/nikhilunni/Projects/melee-lanes/chars/crates/ft-fox)
    Checking ft-yoshi v0.0.1 (/Users/nikhilunni/Projects/melee-lanes/chars/crates/ft-yoshi)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 26.46s
```

### support-final

```sh
cargo test -p melee-test-support
```
Exit 0; wall time 2.0s.

Sum of reported test results: 2 passed, 0 failed, 0 ignored.

```text
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

   Doc-tests melee_test_support

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### gate-final

```sh
cargo gate
```
Exit 101; wall time 34.97s.

Sum of reported test results: 42 passed, 1 failed, 0 ignored.

```text
missing oracle data: /Users/nikhilunni/Projects/melee-lanes/chars/crates/gekko-math/../../harness/traces/jit/frsqrte_probe.jsonl; restore with `GEKKO_PROBE_OUT=harness/traces/jit harness/gekko_probe/run.sh 4`; only contributors without local data may opt out with MELEE_ALLOW_MISSING_DATA=1

failures:
    full_captures_replay_bit_for_bit

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

error: test failed, to rerun pass `-p gekko-math --test golden`
```

### rendered-final

```sh
cargo test -p melee-ft --test start_fox_bones_130 start_fox_matrices_vi_130 -- --exact --nocapture
```
Exit 101; wall time 0.91s.

```text
Compiling melee-ft v0.0.1 (/Users/nikhilunni/Projects/melee-lanes/chars/crates/melee-ft)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.86s
     Running tests/start_fox_bones_130.rs (target/debug/deps/start_fox_bones_130-972098d21a0ca47b)

running 1 test

thread 'start_fox_matrices_vi_130' (42371399) panicked at crates/melee-ft/tests/fighter_support/rendered_pose.rs:18:9:
missing oracle data: /Users/nikhilunni/Projects/melee-lanes/chars/crates/melee-ft/../../harness/traces/start_fd_fox.bones_vi_p0.jsonl; restore with the bone capture command in docs/M2_GATE.md with MELEE_BONES_ANY_ANIM=1 MELEE_BONES_FRAMES=130 MELEE_BONES_FIGHTER_INDEX=0 MELEE_BONES_SAVESTATE=harness/roms/start_fd_fox.sav MELEE_BONES_OUT=harness/traces/start_fd_fox.bones_vi_p0.jsonl; only contributors without local data may opt out with MELEE_ALLOW_MISSING_DATA=1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test start_fox_matrices_vi_130 ... FAILED

failures:

failures:
    start_fox_matrices_vi_130

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p melee-ft --test start_fox_bones_130`
```

### m2-final

```sh
cargo test -p melee-sim --test m2_gate -- --nocapture
```
Exit 101; wall time 0.1s.

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running tests/m2_gate.rs (target/debug/deps/m2_gate-efe5c23cc846ef4a)

running 1 test

thread 'fox_wait1_bones_match_the_real_game_bit_for_bit' (42371465) panicked at crates/melee-sim/tests/m2_gate.rs:39:9:
missing oracle data: /Users/nikhilunni/Projects/melee-lanes/chars/crates/melee-sim/../../harness/traces/fox_ys.bones.expected.jsonl; restore with the bone capture command in docs/M2_GATE.md with MELEE_BONES_SAVESTATE=harness/roms/idle_ys_fox.sav and MELEE_BONES_OUT=harness/traces/fox_ys.bones.expected.jsonl; only contributors without local data may opt out with MELEE_ALLOW_MISSING_DATA=1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test fox_wait1_bones_match_the_real_game_bit_for_bit ... FAILED

failures:

failures:
    fox_wait1_bones_match_the_real_game_bit_for_bit

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p melee-sim --test m2_gate`
```

### missing-strict-final

```sh
MELEE_TEST_DATA_ROOT=/tmp/melee-c11-empty cargo test -p melee-sim --test m4_gate squat_fd_fox_300 -- --exact --nocapture
```
Exit 101; wall time 0.09s.

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/m4_gate.rs (target/debug/deps/m4_gate-095b22569b3296e5)

running 1 test

thread 'squat_fd_fox_300' (42371531) panicked at crates/melee-sim/tests/m4_gate.rs:7:9:
missing oracle data: /tmp/melee-c11-empty/roms/files/PlCo.dat; restore with the disc extraction/savestate instructions in docs/DISC.md; only contributors without local data may opt out with MELEE_ALLOW_MISSING_DATA=1
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test squat_fd_fox_300 ... FAILED

failures:

failures:
    squat_fd_fox_300

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 260 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p melee-sim --test m4_gate`
```

### missing-allowed-final

```sh
MELEE_TEST_DATA_ROOT=/tmp/melee-c11-empty MELEE_ALLOW_MISSING_DATA=1 cargo test -p melee-sim --test m4_gate squat_fd_fox_300 -- --exact --nocapture
```
Exit 0; wall time 0.09s.

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running tests/m4_gate.rs (target/debug/deps/m4_gate-095b22569b3296e5)

running 1 test
missing oracle data: /tmp/melee-c11-empty/roms/files/PlCo.dat; restore with the disc extraction/savestate instructions in docs/DISC.md; only contributors without local data may opt out with MELEE_ALLOW_MISSING_DATA=1
test squat_fd_fox_300 ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 260 filtered out; finished in 0.00s
```

### all-allowed

```sh
MELEE_TEST_DATA_ROOT=/tmp/melee-c11-empty MELEE_ALLOW_MISSING_DATA=1 cargo gate --no-fail-fast
```
Exit 0; wall time 39.12s.

Sum of reported test results: 954 passed, 0 failed, 3 ignored.

```text
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.42s

   Doc-tests slp

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### fmt

```sh
cargo fmt --all
```
Exit 0; wall time 2.61s.

```text

```

### fmt-check

```sh
cargo fmt --all -- --check
```
Exit 0; wall time 2.37s.

```text

```

## Changed files

Protected data/decomp paths and `crates/melee-ft/src` were not edited. The decomp symlink type-change was already present at the start and is excluded from this change list.

```text
CLAUDE.md
Cargo.lock
Cargo.toml
TRACKER.md
crates/ft-captain/Cargo.toml
crates/ft-captain/tests/attributes.rs
crates/ft-falco/Cargo.toml
crates/ft-falco/tests/attributes.rs
crates/ft-fox/Cargo.toml
crates/ft-fox/tests/attributes.rs
crates/ft-mars/Cargo.toml
crates/ft-mars/tests/attributes.rs
crates/ft-peach/Cargo.toml
crates/ft-peach/tests/attributes.rs
crates/ft-purin/Cargo.toml
crates/ft-purin/tests/attributes.rs
crates/ft-yoshi/Cargo.toml
crates/ft-yoshi/tests/attributes.rs
crates/gekko-math/Cargo.toml
crates/gekko-math/tests/golden.rs
crates/gekko-math/tests/ref_oracle.rs
crates/hsd-anim/Cargo.toml
crates/hsd-anim/tests/anim_fobj.rs
crates/hsd-anim/tests/anim_ref_oracle.rs
crates/hsd-anim/tests/mtx_oracle.rs
crates/hsd-anim/tests/real_fox.rs
crates/hsd-archive/Cargo.toml
crates/hsd-archive/tests/real_dat.rs
crates/hsd-particle/Cargo.toml
crates/hsd-particle/tests/live_fd.rs
crates/hsd-particle/tests/live_fd_start.rs
crates/hsd-particle/tests/real_fd_bank.rs
crates/hsd-particle/tests/real_fd_particles.rs
crates/hsd-particle/tests/support/dust_replay.rs
crates/melee-ft/Cargo.toml
crates/melee-ft/tests/airborne_ref_oracle.rs
crates/melee-ft/tests/animation_ref_oracle.rs
crates/melee-ft/tests/fighter_support/mod.rs
crates/melee-ft/tests/fighter_support/rendered_pose.rs
crates/melee-ft/tests/fighter_support/replay.rs
crates/melee-ft/tests/grounded_physics_native.rs
crates/melee-ft/tests/human_idle_input_600.rs
crates/melee-ft/tests/idle_fox_600.rs
crates/melee-ft/tests/idle_ground_fields_600.rs
crates/melee-ft/tests/input_oracle.rs
crates/melee-ft/tests/movement_fox_states.rs
crates/melee-ft/tests/real_fox_data.rs
crates/melee-ft/tests/real_fox_wait_playback.rs
crates/melee-ft/tests/start_fox_states.rs
crates/melee-gr/Cargo.toml
crates/melee-gr/tests/real_battlefield.rs
crates/melee-gr/tests/real_fd.rs
crates/melee-gr/tests/real_pupupu.rs
crates/melee-gr/tests/real_story.rs
crates/melee-lb/Cargo.toml
crates/melee-lb/tests/dynamics_ref_oracle.rs
crates/melee-lb/tests/real_fox_wait.rs
crates/melee-lb/tests/ref_oracle.rs
crates/melee-mp/Cargo.toml
crates/melee-mp/tests/geom_oracle.rs
crates/melee-mp/tests/real_stage.rs
crates/melee-sim/Cargo.toml
crates/melee-sim/src/countdown.rs
crates/melee-sim/src/frame.rs
crates/melee-sim/src/frame/combat.rs
crates/melee-sim/src/frame/falco_bones.rs
crates/melee-sim/src/frame/falcon_bones.rs
crates/melee-sim/src/frame/fall_states.rs
crates/melee-sim/src/frame/marth_bones.rs
crates/melee-sim/src/frame/peach_bones.rs
crates/melee-sim/src/frame/puff_bones.rs
crates/melee-sim/src/frame/puff_state.rs
crates/melee-sim/src/frame/yoshi_bones.rs
crates/melee-sim/src/frame/yoshi_state.rs
crates/melee-sim/src/initial_state/cold_tests.rs
crates/melee-sim/src/scenario.rs
crates/melee-sim/tests/alloc_gate.rs
crates/melee-sim/tests/fixture_spawns.rs
crates/melee-sim/tests/m2_gate.rs
crates/melee-sim/tests/m3_gate.rs
crates/melee-sim/tests/m4_gate.rs
crates/melee-sim/tests/m5_gate.rs
crates/melee-sim/tests/real_fox_bones.rs
crates/melee-sim/tests/slippi_oracle.rs
crates/melee-sim/tests/slippi_replay.rs
crates/melee-test-support/Cargo.toml
crates/melee-test-support/src/lib.rs
crates/melee-test-support/tests/policy.rs
docs/DOLPHIN_RUN.md
docs/PORT_NOTES/C11_NO_SILENT_PASS.md
tools/merge-check.sh
tools/tests/test_merge_check.py
```
