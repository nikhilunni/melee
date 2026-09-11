# Reusable match library extraction

`melee-lib` owns match composition and a curated Rust API for creation, one-tick
advancement, borrowed observation, ordinary cloning, and reset. `melee-sim`
retains CLI/scenario/Slippi parsing, scripted input playback, trace output, and
oracle comparisons. No macOS, C ABI, or rendering implementation was added.

Immutable game resources and effect definitions are shared. Scheduler storage,
character payloads, animation state, particle ownership, pending work, RNG, and
input state belong to each match. Cloning preserves prepared capacities; the
tagged scheduler shares ordering machinery with the existing general scheduler
without inheriting its owned closures or arbitrary object payloads.

The application API and oracle adapter intentionally use different startup
indices. Public construction completes the pre-music/reset boundary without
advancing gameplay; its first step is a full tick. Oracle tooling retains the
existing partial-boundary tick zero. The cold-start seed boundary is unchanged.

The examples use only the public API. `headless` supplies controller samples and
prints the resulting state. `training` owns its objective, compares four-tick
cloned continuations, selects inputs, and resets episodes.

Performance census coverage now includes melee-lib. Both composition crates
share the old combined duplicate-label budget, without a second allowance;
subsystem, cross-crate, timing, and exactness limits remain unchanged.
The user explicitly deferred binary-size optimization on 2026-09-10; size
measurements and the existing gate failures are retained as evidence, rather
than silently treating a failed size gate as a pass.
A regression test verifies this budget transfer and rejects increases.

## Verification

- `RUST_TEST_THREADS=2 cargo gate`: 1,144 passed, 0 failed, 3 existing ignored.
- `RUST_TEST_THREADS=2 cargo gate --release`: 1,144 passed, 0 failed, 3 existing ignored.
- Both profiles preserve `match_fd_foxmarth_6083_ticks_and_ordered_particle_draws`
  and `match2_fd_foxmarth_10059_ticks_and_ordered_particle_draws` unchanged.
- All ten requested API behavior tests passed, alongside three allocation tests:
  full-match stepping after clone/clone_from, cold construction stepping and
  observation, and complete release of match/resource allocations on drop.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all` and `git diff --check`: clean.
- Performance-guard Python tests: 12 passed.
- Both release examples ran successfully: headless reached tick 600; training
  evaluated cloned continuations and reset three 180-tick episodes.
- Final `tools/perf-gate.sh`: timing and duplicate-definition checks passed;
  command exits 1 solely for the existing size ceilings, deferred by the user.
  Load: 179.722 ms; 600 simulate-only ticks: 24.643 ms; stripped binary:
  3,757,952 bytes; text: 3,424,256 bytes; cross-crate duplicate labels: 100.
  No timing, allocation, duplicate-definition, or exactness allowance was loosened.
  Size thresholds remain unchanged so the deferred work stays visible in the gate.
  Earlier measurements and final evidence are retained in `docs/PERF.md`.

## Lifecycle measurements

Release, local Apple Silicon machine, 50 samples after 180 neutral ticks;
immutable asset loading excluded. These are descriptive measurements, not gates.

| Operation | Mean per match |
|---|---:|
| Creation plus drop | 1.425 ms |
| Reset | 1.436 ms |
| Clone plus drop | 1.603 ms |
| clone_from | 1.260 ms |

Incremental retained heap was 21,350,275 bytes per match: 21.35 MB for one,
170.80 MB for eight, and 683.21 MB for 32 matches sharing assets. This includes
prepared simulation capacities, excludes shared assets and allocator overhead,
and is a useful future optimization target for large training worker counts.

## Limits

Only the existing two-fighter stock composition is exposed. Registered moves
and stages retain their existing implementation coverage. Two pre-existing
ignored throw scratch comparisons and an illustrative schema doctest remain
ignored, with their original reasons.
`clone_from` currently replaces storage rather than promising allocation reuse.
The unstable diagnostics/import API is for oracle tools, not application state
editing. The graphical application is a later milestone.

## Changed files

Production modules moved from `melee-sim` to `melee-lib` appear as removed and
added paths in this uncommitted checkout. No game data or decomp paths changed.

- `CLAUDE.md`
- `Cargo.lock`
- `Cargo.toml`
- `TRACKER.md`
- `crates/hsd-anim/src/aobj.rs`
- `crates/hsd-anim/src/jobj.rs`
- `crates/hsd-gobj/src/lib.rs`
- `crates/hsd-gobj/src/slab.rs`
- `crates/hsd-gobj/src/world.rs`
- `crates/hsd-gobj/tests/tagged_clone.rs`
- `crates/hsd-particle/src/rng_sites.rs`
- `crates/hsd-particle/src/system.rs`
- `crates/hsd-types/src/lib.rs`
- `crates/hsd-types/src/storage.rs`
- `crates/melee-coll/src/damage.rs`
- `crates/melee-ef/src/fixture_spawns.rs`
- `crates/melee-ef/src/lib.rs`
- `crates/melee-ef/src/pool.rs`
- `crates/melee-ef/src/resources.rs`
- `crates/melee-ef/src/tests.rs`
- `crates/melee-ft/src/anim/playback.rs`
- `crates/melee-ft/src/fighter/attack/combo.rs`
- `crates/melee-ft/src/fighter/attack/stale.rs`
- `crates/melee-ft/src/fighter/character.rs`
- `crates/melee-ft/src/fighter/character/tests.rs`
- `crates/melee-ft/src/fighter/damage.rs`
- `crates/melee-ft/src/fighter/mod.rs`
- `crates/melee-ft/src/fighter/spawn.rs`
- `crates/melee-gr/src/last/animation.rs`
- `crates/melee-gr/src/last/mod.rs`
- `crates/melee-if/src/lib.rs`
- `crates/melee-it/src/engine.rs`
- `crates/melee-lb/src/radial_force.rs`
- `crates/melee-lib/Cargo.toml`
- `crates/melee-lib/README.md`
- `crates/melee-lib/benches/lifecycle.rs`
- `crates/melee-lib/examples/headless.rs`
- `crates/melee-lib/examples/training.rs`
- `crates/melee-lib/src/assets.rs`
- `crates/melee-lib/src/config.rs`
- `crates/melee-lib/src/countdown.rs`
- `crates/melee-lib/src/diagnostics.rs`
- `crates/melee-lib/src/diagnostics/items.rs`
- `crates/melee-lib/src/diagnostics/snapshot.rs`
- `crates/melee-lib/src/error.rs`
- `crates/melee-lib/src/frame.rs`
- `crates/melee-lib/src/frame/combat.rs`
- `crates/melee-lib/src/frame/falco_bones.rs`
- `crates/melee-lib/src/frame/falcon_bones.rs`
- `crates/melee-lib/src/frame/fall_states.rs`
- `crates/melee-lib/src/frame/fd_background.rs`
- `crates/melee-lib/src/frame/grab_pairs.rs`
- `crates/melee-lib/src/frame/marth_bones.rs`
- `crates/melee-lib/src/frame/peach_bones.rs`
- `crates/melee-lib/src/frame/puff_bones.rs`
- `crates/melee-lib/src/frame/puff_state.rs`
- `crates/melee-lib/src/frame/rendered_pose.rs`
- `crates/melee-lib/src/frame/yoshi_bones.rs`
- `crates/melee-lib/src/frame/yoshi_state.rs`
- `crates/melee-lib/src/game.rs`
- `crates/melee-lib/src/initial_state/cold.rs`
- `crates/melee-lib/src/initial_state/cold_tests.rs`
- `crates/melee-lib/src/initial_state/collision.rs`
- `crates/melee-lib/src/initial_state/fighter.rs`
- `crates/melee-lib/src/initial_state/mod.rs`
- `crates/melee-lib/src/initial_state/particle_resume.rs`
- `crates/melee-lib/src/initial_state/particles.rs`
- `crates/melee-lib/src/initial_state/saved_pose.rs`
- `crates/melee-lib/src/initial_state/scheduler_resume.rs`
- `crates/melee-lib/src/initial_state/setup_resume.rs`
- `crates/melee-lib/src/initial_state/stage.rs`
- `crates/melee-lib/src/initial_state/stock.rs`
- `crates/melee-lib/src/input.rs`
- `crates/melee-lib/src/lib.rs`
- `crates/melee-lib/src/observation.rs`
- `crates/melee-lib/src/scene_fighter.rs`
- `crates/melee-lib/src/scene_items.rs`
- `crates/melee-lib/src/scene_stage.rs`
- `crates/melee-lib/src/scene_stage/last.rs`
- `crates/melee-lib/src/scene_stage/pupupu.rs`
- `crates/melee-lib/src/setup.rs`
- `crates/melee-lib/tests/allocation.rs`
- `crates/melee-lib/tests/api.rs`
- `crates/melee-sim/Cargo.toml`
- `crates/melee-sim/src/assets.rs`
- `crates/melee-sim/src/countdown.rs`
- `crates/melee-sim/src/frame.rs`
- `crates/melee-sim/src/frame/combat.rs`
- `crates/melee-sim/src/frame/falco_bones.rs`
- `crates/melee-sim/src/frame/falcon_bones.rs`
- `crates/melee-sim/src/frame/fall_states.rs`
- `crates/melee-sim/src/frame/fd_background.rs`
- `crates/melee-sim/src/frame/grab_pairs.rs`
- `crates/melee-sim/src/frame/marth_bones.rs`
- `crates/melee-sim/src/frame/peach_bones.rs`
- `crates/melee-sim/src/frame/puff_bones.rs`
- `crates/melee-sim/src/frame/puff_state.rs`
- `crates/melee-sim/src/frame/rendered_pose.rs`
- `crates/melee-sim/src/frame/yoshi_bones.rs`
- `crates/melee-sim/src/frame/yoshi_state.rs`
- `crates/melee-sim/src/initial_state.rs`
- `crates/melee-sim/src/initial_state/cold.rs`
- `crates/melee-sim/src/initial_state/cold_tests.rs`
- `crates/melee-sim/src/initial_state/collision.rs`
- `crates/melee-sim/src/initial_state/fighter.rs`
- `crates/melee-sim/src/initial_state/mod.rs`
- `crates/melee-sim/src/initial_state/particle_resume.rs`
- `crates/melee-sim/src/initial_state/particles.rs`
- `crates/melee-sim/src/initial_state/saved_pose.rs`
- `crates/melee-sim/src/initial_state/scheduler_resume.rs`
- `crates/melee-sim/src/initial_state/setup_resume.rs`
- `crates/melee-sim/src/initial_state/stage.rs`
- `crates/melee-sim/src/initial_state/stock.rs`
- `crates/melee-sim/src/lib.rs`
- `crates/melee-sim/src/replay.rs`
- `crates/melee-sim/src/scenario.rs`
- `crates/melee-sim/src/scene_fighter.rs`
- `crates/melee-sim/src/scene_items.rs`
- `crates/melee-sim/src/scene_stage.rs`
- `crates/melee-sim/src/scene_stage/last.rs`
- `crates/melee-sim/src/scene_stage/pupupu.rs`
- `crates/melee-sim/src/trace.rs`
- `crates/melee-sim/src/trace_items.rs`
- `crates/melee-sim/tests/alloc_gate/revival.rs`
- `docs/PERF.md`
- `docs/PORT_NOTES/MELEE_LIB.md`
- `tools/perf-gate.sh`
- `tools/perf_report.py`
- `tools/tests/test_perf_gate.py`
