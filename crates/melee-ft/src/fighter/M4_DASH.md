# M4-T2: Fox dash on Final Destination

Implemented and verified, left uncommitted as requested. Dash -> Run ->
RunBrake -> Wait occurs at ticks **31 -> 42 -> 56 -> 74**. The full gate
reports **300 ticks, 49 keys, 0 divergences**. All 300 ordered particle RNG
site lists match retail independently of the seed comparison.

## What changed

- Typed movement callbacks/scratch data, PlCo running parameters, dash initial
  acceleration and directional redash, Run acceleration/animation scaling,
  RunBrake friction/duration/animation controls, ordinary ground collision and
  explicit unsupported interaction branches.
- Five-word GFX decoding, command variables, rotating bone selection,
  destroy-on-state-change flag, invisible-command suppression, TransN setup,
  flat-floor brake body tilt, and initial-boundary import of newly owned fields.
- Async position/parameter dispatch, effect-table lookup, joint-relative offset
  transformation, and static shared AppSRT inheritance/lifetime for dust.
- Named RNG sites with gating conditions, full-scene ordered-ledger regression,
  callback scratch/flag replay, and focused effect/AppSRT behavior tests.

## Actual dust routing and corrections to the task notes

| Tick | GFX ID | Async kind | Effect / particle kinds (bank 0) |
|---|---|---|---|
| 34 | 0x3FF | 6 | effect-table entry 5 -> DPtcl 9 -> A5 children 7, 8 |
| 49 | 0x3FE | 5 | generator 263 -> EF children 264, 265 (blend 7) |
| 56 | 0x401 | 5 | generator 90 |

These IDs come from the owned animation scripts, efasync.c dispatch and
EfCoData particle programs. They do not use the early direct-ID kind-2 branch.
The requested kind-2 position-offset route is also implemented for direct
particle descriptors in the supplied bank. Unsupported/missing banks fail
explicitly.

The ledger's fighter draw branches FCDC/FD00/FD24 are in the later switch.
The corresponding additions are **fmadds at 8009FCF8/FD1C/FD44**. The early
branch uses **8009F94C/F970/F9A4**. Range doubling and random-minus-half are
separately rounded before each fused addition. The subaction scale is retail's
literal **0.003906f**, not an exact binary 1/256.

All seven particle descriptors use shape 0. The task's listed emitter and
bytecode RNG paths were already audited and implemented; the missing particle
behavior was shared AppSRT inheritance. No new emitter shape or speculative
particle fields were added. The canonical coverage doc is `docs/PARTICLES.md`;
`crates/hsd-particle/docs/PARTICLES.md` does not exist.

## Verification

Commands ran from the repository root, with output logs under `/tmp`.
The table records the final nonblank output lines (timings are from this run).

| Command | Final output |
|---|---|
| `cargo run -q -p melee-sim -- gate harness/scenarios/dash_fd_fox.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/idle_fd_fox.toml` | `600 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_fox.toml` | `600 ticks, 49 keys, 0 divergences` |
| `cargo test -p melee-sim --test m4_gate` | `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.37s` |
| `cargo test -p melee-ft --test movement_fox_states` | `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s` |
| `cargo test -p melee-ft --test start_fox_bones_130` | `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.72s` |
| `cargo test -p hsd-particle --test live_fd` | `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.03s` |
| `cargo test -p melee-sim --test m2_gate` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s` |
| `cargo test -p melee-ft --test fighter_graphics` | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s` |
| `cargo test -q -p hsd-particle --test start_paths` | `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` |
| `cargo gate` | `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` (last crate's doctests) |
| `cargo clippy --workspace --all-targets -- -D warnings` | ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.76s`` |
| `cargo fmt --all` | success, no output |
| `git -c core.fsmonitor=false diff --check` | success, no output |

Final `cargo gate` totals: **555 passed, zero failed, one pre-existing ignored
doctest**, across 100 suite result lines. The initial baseline was green too.
The final gate includes the latest collision guard, command flag test and all
named regressions. No expectations or tolerances changed; local assets were
present, so the oracle tests ran.

Read-only harness commands, run in `harness` with
`UV_CACHE_DIR=/tmp/melee-uv-cache`:

```sh
uv run python rng_ledger_report.py traces/dash_fd_fox.ledger.raw.jsonl --ticks 300
uv run python asm.py ftCo_8009F834 --fused
uv run python asm.py ftCo_Dash_Phys ftCo_Run_Phys ftCo_Dash_IASA ftCommon_800804A0 ftCo_RunBrake_Phys ftAction_80071028 --fused
uv run python asm.py hsd_8039930C hsd_8039DAD4 hsd_8039F05C --fused
uv run python asm.py ft_80089B08 --fused
uv run python asm.py ftAction_80071820
uv run python asm.py ftCo_Dash_Enter ftCo_Dash_Anim ftCo_Run_Anim ftCo_RunBrake_Anim efLib_CreateGenerator_Translate_FacingDir --fused
```

The ledger reports **300 records / 9,080 draws**. The dust spawner audit ends
with `8009FD44 EC02007A fmadds f0, f2, f1, f0`. The Dash IASA has the separate
interrupt-friction fusion at 800CA51C; Dash/Run physics, entry/rate/brake
callbacks, GFX decoding and the facing-transform constructor have no fused
sites. Flat-floor body tilt's squared line length uses 8008A028 fmadds.
Existing particle fusion sites are documented in `docs/PARTICLES.md`.

Intermediate failures were missing TransN setup, a reversed squat comparison
in the new brake IASA, an unsupported effect-kind allowlist entry, shared
AppSRT inheritance, and flat-floor body tilt. These were fixed from C/asm;
no oracle expectations were edited.

## Limits

No dash particle dump was supplied or needed to establish the requested
49-key and ordered-RNG gates. Dust particle positions/velocities are derived
from the owned data and C/asm, but lack an independent per-field retail dump.
Existing idle and match-start particle-field replays remain green. Rendering,
mutable/descriptor-created AppSRTs, unsupported interactions and other stages
remain outside this slice. No Dolphin invocation, game-data changes, protected
path changes, commits or pushes. Harness tests were not run because no harness
code changed.

## Files touched

- `TRACKER.md`
- `crates/hsd-particle/src/generator.rs`
- `crates/hsd-particle/src/particle.rs`
- `crates/hsd-particle/src/rng_sites.rs`
- `crates/hsd-particle/src/system.rs`
- `crates/hsd-particle/tests/start_paths.rs`
- `crates/melee-ft/src/collision/pose.rs`
- `crates/melee-ft/src/fighter/M4_DASH.md`
- `crates/melee-ft/src/fighter/README.md`
- `crates/melee-ft/src/fighter/assets.rs`
- `crates/melee-ft/src/fighter/commands.rs`
- `crates/melee-ft/src/fighter/dash.rs`
- `crates/melee-ft/src/fighter/effects.rs`
- `crates/melee-ft/src/fighter/mod.rs`
- `crates/melee-ft/src/fighter/procs.rs`
- `crates/melee-ft/src/fighter/run.rs`
- `crates/melee-ft/src/fighter/spawn.rs`
- `crates/melee-ft/src/fighter/state.rs`
- `crates/melee-ft/src/fighter/turn.rs`
- `crates/melee-ft/src/fighter/walk.rs`
- `crates/melee-ft/tests/fighter_graphics.rs`
- `crates/melee-ft/tests/fighter_support/mod.rs`
- `crates/melee-ft/tests/fighter_support/replay.rs`
- `crates/melee-ft/tests/movement_fox_states.rs`
- `crates/melee-sim/src/effects.rs`
- `crates/melee-sim/src/effects/dust.rs`
- `crates/melee-sim/src/frame.rs`
- `crates/melee-sim/src/initial_state/fighter.rs`
- `crates/melee-sim/tests/m4_gate.rs`
- `docs/PARTICLES.md`

## Particle-dump follow-up (2026-09-09, incomplete)

Added `hsd-particle/tests/live_fd_dash.rs` with production spawn inputs in
`tests/support/dash_fd_spawns.rs`: restore the initial snapshot and attached
stage matrix, then compare **442,389 fields over all 300 ticks** and **9,066
ordered particle RNG draws**. Every generator/particle field, AppSRT SRT,
reference count and family ID matches. No dust routing, blend, life, position
or velocity correction was needed. The first harness mistake was omitting the
initial stage joint matrix (tick 0 particle X differed by one ULP); restoring
that input fixed it. The first adapter gap was absent AppSRT fields at tick 49;
typed AppSRT cache state, normalized identities and live-owner counts fix that.

**The strict test still fails: 425 display-cache differences, first at tick 50
`particles.appsrt[0].frame_count` (retail 185, port 0).** The dump lacks the
external display inputs: active camera view matrix and `psFrameNum`, including
the display-pass order between tick samples. The camera subsystem is not
ported. These cannot be reconstructed bit-exactly by feeding expected AppSRT
output back as input. `psDispSubAppSRT` updates the cached model/model-view
matrix, axis lengths, status and frame number (psdisp.c:1400-1439).
`prepare_application_transforms` now ports that operation with explicit view
and frame inputs; a synthetic test checks shared caches and frame reuse/wrap.
Its sums are unfused at 803A1FFC..2014/2078..2090; sqrt Newton steps fuse at
803A202C/203C/204C. Audit: `cd harness && UV_CACHE_DIR=/tmp/melee-uv-cache uv run
python asm.py psDispParticles --fused` (also inspected the full disassembly).
The replay deliberately supplies no guessed view or captured cache outputs.
A display-input capture is required to finish; no comparisons are excluded.
Dust spawns are at **34/49/56**, not 32-34.

Validation this session (logs `/tmp/melee-dash-particles-*.log`):

| Command | Result |
|---|---|
| `cargo test -p hsd-particle --test live_fd_dash -- --nocapture` | **FAIL**, 442389 comparisons, 425 AppSRT display-cache differences |
| `cargo test -p melee-sim --test m4_gate` | 5 passed |
| `cargo run -q -p melee-sim -- gate harness/scenarios/idle_fd_fox.toml` | `600 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_fox.toml` | `600 ticks, 49 keys, 0 divergences` |
| `cargo test -p hsd-particle --test live_fd` | 2 passed |
| `cargo test -p hsd-particle --test live_fd_start` | 2 passed |
| `cargo test -p hsd-particle --test start_paths` | 9 passed |
| `cargo gate` | Baseline passed; final fails in the new strict dash replay |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --all` / `git -c core.fsmonitor=false diff --check` | success |

No protected captures/data/decomp changed, no Dolphin invocation, no commit.

Files touched in this follow-up:

- `crates/hsd-particle/src/{appsrt.rs,generator.rs,lib.rs,system.rs}`
- `crates/hsd-particle/tests/{live_fd_dash.rs,start_paths.rs,support/restore.rs,support/dash_fd_spawns.rs}`
- `crates/melee-sim/src/{effects/dust.rs,initial_state/particles.rs}`
- `crates/melee-ft/src/fighter/M4_DASH.md`, `docs/PARTICLES.md`, `TRACKER.md`
