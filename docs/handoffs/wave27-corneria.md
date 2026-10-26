# Corneria, items on Brinstar/Venom/Corneria: ported, verification unfinished

Branch `worktree-agent-a1c35ef88761f969e` holds one `WIP:` commit on top of
`main` 208d7a84. Its contents:

- Corneria (grcorneria.c, GrCn.dat, StKind 0x07): the Great Fox's drift,
  descent and splashes, the cannon and its material items (break at 200 beam
  damage), the Great Fox laser (`crates/it-greatfoxlaser`), the scrolling
  backgrounds, the Arwing/Wolfen spawner and passes, level and aimed lasers
  (`it-arwinglaser` types 0, 2, 4, 5 added). Notes in
  `docs/PORT_NOTES/WAVE27_STAGES.md`. Boundary `start_cor_fox_marth4`
  registered (`harness/boundaries.toml`, `m4_gate`, `cold_tests`).
- `Stage::items_supported` admits Brinstar, Venom and Corneria. There are new
  item boundaries `w27_start_{Brinstar,Venom,Corneria}_items_all`, and an
  opened Barrel's staves are reset to rest (Item_80268D34 lb_8000B804).
- A Peach turnip's doAirEnd0 against an item target
  (`crates/ft-peach/src/special_s.rs`) now returns where it used to be
  guarded.
- Registrations in `m5_gate.rs`:
  - `CORPUS_V3_MATCHES`: 13 new entries.
  - `MATRIX_WITNESSES`: `w27_cor_cannon_break`.
  - `WIDENED_LAYOUT_BOUNDARIES`: 6 new entries.
  - `WIDENED_LAYOUT_MATCHES`: 6 `w27i_*` entries.
  - The scenario TOMLs are committed. Their traces are in the main checkout.

## Before the rebase (verified)

Every registered scenario passed gate exactly on the pre-rebase branch (base
928af3a8) in the development runs. The registrations themselves were added
after those runs and were never run through `m5_gate` before the rebase.

## Rebase onto 208d7a84 (not verified)

`main` had added `InitialState::stage_limits`
(`crates/melee-lib/src/stage_limits.rs`, which Mute City mutates), and the
branch had added `scene_stage::live_camera(stage, assets)` and
`live_arena`. The resolution:

- `live_arena` is gone. Corneria's proc (`scene_stage/corneria/great_fox.rs`
  `tick`) now writes `state.stage_limits = corneria.limits(map_scale)`, which
  includes the lowest eye height (grCorneria_801E2FCC). `StageLimits::of`
  gives the limits at creation for the cold and warm starts.
- `live_camera(stage, &StageCamera)` keeps `main`'s signature (only Brinstar
  adjusts it). The branch's call sites keep reading it, wrapped around
  `state.stage_limits.camera`: the CPU scene, offscreen damage, fighter
  dispatch, revival, and `Simulation::stage_camera`.
- `assets::stage_limits` (the per-tick marker read) now carries `main`'s
  Rainbow Cruise revival flags and `tracked_revival_markers`. Ice Mountain's
  revivals and Poké Floats' camera shrink stay with the creation caller.

`cargo check --workspace --all-targets --release` and
`cargo clippy --workspace --all-targets -- -D warnings` are clean. The
release `m5_gate` run had started (it needs
`MELEE_DATA_ROOT=/Users/nikhilunni/Projects/melee`) but had not finished
when the wave closed.

## Remaining

1. Run `MELEE_DATA_ROOT=... cargo test --release -p melee-sim --test m5_gate`
   in full. Run `m4_gate` with `start_cor` and the Brinstar/Venom start
   tests. Run the `melee-lib` tests (`cold_tests`, `allocation`).
2. Sweep against `target/sweeps/e7e3a37d.tsv` (or the current `main`
   baseline) with `--lock`.
3. `replay-batch`: `ft-peach` and `hsd-particle` are shared code that
   changed.
4. If everything is exact, reword the commit without `WIP:`.
5. Still guarded: a reflected level cannon (itarwinglaser.c:576), an
   absorbed ship laser, a reflected Great Fox shot (itgreatfoxlaser.c:148),
   warm boundaries with a pass out or a charging or broken cannon, and
   ftCo_800DC920's captured-fighter release.
