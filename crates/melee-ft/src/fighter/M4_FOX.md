# M4-T1 Fox grounded movement report — 2026-09-09

Squat/SquatWait/SquatRv, Turn and WalkSlow/Middle/Fast are implemented with
state-family modules, typed scratch data and enum callbacks. The three retail
movement gates pass all 300 ticks and 49 keys. All regression gates pass.
Nothing was committed, Dolphin was not run, and no protected harness data,
scenario or decomp files were modified. Oracle expectations were not changed.

The port adds walk acceleration/target clamping to shared grounded physics,
ordinary ground collision for crouch/turn, per-state IASA order, immediate
entry animation steps, speed-dependent animation rates and phase conversion,
script goto/animation-loop waits and nonzero-phase seeking. Footstep sounds
and controller rumble remain typed output requests.

## Final validation

All commands below ran from the repository root and exited zero. Final
nonblank output lines are transcribed below. `cargo gate` totals **550 passed,
0 failed, 1 pre-existing ignored doctest**, across 99 result blocks; its last
line alone is the final empty doctest target. The baseline `cargo gate` also
passed before edits (543 passed, 0 failed, 1 ignored).

| Exact command | Final nonblank line |
|---|---|
| `cargo run -q -p melee-sim -- gate harness/scenarios/squat_fd_fox.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/turn_fd_fox.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/walk_fd_fox.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/idle_fd_fox.toml` | `600 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_fox.toml` | `600 ticks, 49 keys, 0 divergences` |
| `cargo test -p melee-ft --test movement_fox_states` | `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s` |
| `cargo test -p melee-sim --test m4_gate` | `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.35s` |
| `cargo test -p melee-ft --test start_fox_bones_130` | `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.72s` |
| `cargo test -p hsd-particle --test live_fd` | `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.04s` |
| `cargo test -p melee-sim --test m2_gate` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s` |
| `cargo gate` | `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` |
| `cargo clippy --workspace --all-targets -- -D warnings` | `Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.10s` |
| `cargo fmt --all` | No output; exit 0. |
| `git -c core.fsmonitor=false diff --check` | No output; exit 0. |

The focused tests used the available local retail data. Movement callback
replays also compare raw animation speed/remainder, ground speed, command
clock, nametag countdown, and Turn/Walk/Squat scratch. They use only pads and
explicit external pre-draw seeds as subsequent inputs; the full simulation
gates independently produce the complete RNG stream. The new Fast threshold
test checks exact boundary inclusion for both directions.

Development stops were missing resource opcodes 54 (footstep), 8 (animation
loop wait) and 43 (rumble), and one clippy `while_let_loop` finding; all were
fixed. The three scenario gates had no bit divergences once their resource
commands were supported. No expected values or tolerance were altered.

## Retail assembly audit

The following exact command prefix ran from `harness/`:
`UV_CACHE_DIR=/tmp/melee-m4-uv uv run python asm.py`.

| Arguments after the prefix | Output |
|---|---|
| `ftWalkCommon_800DFEC8 --fused` | `800E0010  EC01183C  fnmsubs f0, f1, f0, f3` |
| `ftWalkCommon_800E0060 --fused` | No fused instructions. |
| `ftCommon_8007C98C --fused` | No fused instructions. |
| `ftWalkCommon_800DFCA4 --fused` | No fused instructions. |
| `ftWalkCommon_800DFDDC --fused` | No fused instructions. |
| `ftCo_Turn_Anim_Inner --fused` | No fused instructions. |
| `ftCo_Squat_Enter --fused` | No fused instructions. |
| `ftCo_Turn_Enter --fused` | No fused instructions. |
| `ftCo_Turn_Enter_Smash --fused` | No fused instructions. |
| `ftWalkCommon_800E0060` | Full assembly inspected; final line `800E0130  4E800020  blr`. |
| `ftWalkCommon_800DFEC8` | Full assembly inspected; final line `800E005C  4E800020  blr`. |

`symbols.py` resolved all callback/entry addresses documented in README.md.
Walk acceleration retains separate rounded multiply/add instructions; phase
conversion uses the audited `fnmsubs` followed by retail-width conversion.

## Files touched

Implementation and verification (paths relative to repository root):

- `crates/melee-ft/src/fighter/{squat,turn,walk}.rs` — new state families.
- `crates/melee-ft/src/fighter/{assets,commands,mod,procs,spawn,state}.rs` — resources, scripts, ownership, dispatch and entry.
- `crates/melee-ft/src/physics/grounded.rs` — shared friction/projection/integration and walk acceleration.
- `crates/melee-ft/src/collision/ground.rs` — ordinary ground map callback.
- `crates/melee-ft/src/desc/{common,playback}.rs` — movement parameters and optional crouch animation choices.
- `crates/melee-ft/tests/fighter_support/replay.rs` — trace-length/ledger-suffix configuration, pads, callback-only mode and raw-field checks.
- `crates/melee-ft/tests/movement_fox_states.rs` — three state replays.
- `crates/melee-sim/tests/m4_gate.rs` — three complete scene gates.
- `crates/melee-ft/src/fighter/{README.md,M4_FOX.md}` — port table, current boundaries and this report.
- `TRACKER.md` — verified, intentionally uncommitted status and session log.

The requested workspace-wide `cargo fmt --all` also changed formatting only in:

- `crates/gekko-math/src/rng.rs`
- `crates/hsd-anim/src/{aobj,lib,load}.rs`
- `crates/hsd-anim/tests/{anim_aobj,anim_fobj,load,real_fox}.rs`
- `crates/hsd-archive/tests/real_dat.rs`
- `crates/hsd-particle/tests/live_fd.rs`
- `crates/melee-diff/src/lib.rs`
- `crates/melee-platform/src/lib.rs`
- `crates/melee-sim/src/inputs.rs`

## Findings and limits

No scenario-note contradictions: the observed state transitions match the
supplied ticks, and no new RNG sites are needed. SquatWait's Fox choice table
is null; its shared completion helper therefore restarts without drawing RNG.
Walk footstep commands on FD select audio only, with no particle request.
The older fighter docs had stale match-start/dynamics blocker statements; the
README now distinguishes those historical reports from current passing gates.

WalkFast uses the common implementation and has threshold coverage, but lacks
a supplied retail scenario. TurnRun, dash/attack/special/jump/shield bodies,
platform-drop entry, ledge transitions and non-default-terrain footstep effects
remain explicit unsupported branches. Controller/audio playback remains an
output-layer responsibility. No requested acceptance check was left undone.
