# M4-T3: Fox jump on Final Destination

Implemented and verified; no commit. `jump_fd_fox` reports **300 ticks,
49 keys, 0 divergences**. All requested regressions, the workspace gate and
clippy pass with local assets present.

## Port

- KneeBend input source and latched release decision; JumpF entry velocity,
  initial gravity skip and ordinary air drift; Fox's basic JumpAerialF entry
  and immediate air physics; fast-fall threshold/window and motion-entry reset.
- Enum callbacks and typed motion scratch; aerial double-jump IASA dispatch;
  direct Landing into SquatWait with retained scratch and nametag behavior.
- Kind-0 GFX dispatch with no offset RNG; live fighter-bone particle attachment
  updates before emission. Existing Landing effects and particle paths suffice.
- Callback replay plus full-hop/release/repress behavior test; full-scene ordered
  RNG checks; shared dust particle replay with production spawn/joint fixture.

All observed transitions agree with the task: 31/34/55/85, then
101/104/113/152/156/177/187. Both ground jumps are short hops. Fast fall begins
at 136 and resets on landing at 152 (the task gives the approximate onset as 135). The first JumpF never reaches Fall.

## Dust and corrections

| Tick | Request | Dispatch | Particle kind (bank 0) |
|---|---|---|---|
| 36, 106 | Jump GFX 0x402 | async kind 0, live fighter root joint | 89 (0x59) |
| 113 | JumpAerial GFX 0x403 | async kind 0, live fighter root joint | 94 (0x5E) |
| 55, 152 | Landing 0x404 | async kind 6, effect-table 0x18 | DPtcl 10 |

The two randomized dust-spawner calls are both **landings**, not one jump and
one landing. Kind-0 jump requests consume no fighter offset RNG. All stated
particle call-site counts match: BD 20, AC 8, radius/azimuth 9 each,
negative-angle pre-loop 4. Fox does not use JumpAerialF1. The current decomp
helpers are `ftCo_800CB110` and `ftCommon_CheckFallFast` (8007D528), rather than
the approximate helper names in the task.

The new field oracle matches **489,588 fields and 9,544 ordered particle draws**
over 300 ticks; final seed **0x059317FC**. It reuses the exact original dash
AppSRT display-cache helper; this jump dump has **zero fields excluded**.
The five external requests and their joint matrices were logged from the
production jump gate, with all temporary logging removed afterwards.
No retail output fields drive the port. The hsd-particle simulation engine
needed no changes: its existing disc/interpreter paths already match.
The shared runner preserves dash's 441,857 field comparisons and 532 documented
display-cache exclusions. Earlier documentation calling that gate red is stale.

Raw callback verification additionally found that direct SquatWait preserves
Landing's two scratch words; the inactive drop timer retains the preceding
Jump physics flag's bit pattern. The port preserves this deliberately; no
expected values or comparison rules were changed.

## Validation

All commands ran from the repository root and exited zero. Logs are under
`/tmp/melee-m4t3-*`, with focused callback/particle logs `/tmp/melee-jump-*`.
The final empty result from `cargo gate` is its last crate's doctest target.

| Exact command | Final nonblank line |
|---|---|
| `cargo run -q -p melee-sim -- gate harness/scenarios/jump_fd_fox.toml` | 300 ticks, 49 keys, 0 divergences |
| `cargo test -p melee-sim --test m4_gate` | test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.50s |
| `cargo run -q -p melee-sim -- gate harness/scenarios/idle_fd_fox.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_fox.toml` | 600 ticks, 49 keys, 0 divergences |
| `cargo test -p melee-ft --test start_fox_bones_130` | test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.86s |
| `cargo test -p hsd-particle --test live_fd` | test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.14s |
| `cargo test -p melee-sim --test m2_gate` | test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s |
| `cargo gate` | test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s |
| `cargo clippy --workspace --all-targets -- -D warnings` | Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 7.31s |
| `cargo test -p melee-ft --test movement_fox_states` | test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.62s |
| `cargo test -p hsd-particle --test live_fd_jump --test live_fd_dash -- --nocapture` | test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.13s |
| `cargo fmt --all` | success, no output |
| `git -c core.fsmonitor=false diff --check` | success, no output |

Baseline `cargo gate`: 557 passed, zero failed,
one pre-existing ignored doctest. Final: **562 passed, zero failed, one pre-existing ignored doctest**.
The combined particle command passes both dash and jump (one test per target).

Retail audits ran read-only from `harness`, with `UV_CACHE_DIR=/tmp/melee-uv-cache`:

```sh
uv run python asm.py ftCo_800CB110 ftCo_Jump_Enter ftCo_JumpAerial_Enter_Basic ftCo_800CBAC4 ftCommon_CheckFallFast ft_80084DB0 ftCommon_FallFast ftCo_KneeBend_Enter ftCo_KneeBend_Anim ftCo_KneeBend_Check_ShortHop --fused
uv run python rng_ledger_report.py traces/jump_fd_fox.ledger.raw.jsonl --ticks 300
```

No fused instructions occur in the audited helpers. Full assembly was also
read for launch ordering; 800CB140/144/174/180/18C/198/1A8/1B8 and
800CBC44/4C retain separate rounding. The ledger reports 300 records and
9,554 total draws; its final tick section ends at
`hsd_8039EE24+0xDC <HSD_Randf>`.

## Limits

Explicit boundaries remain for backward jumps, FallAerial animation completion,
character-specific/multijump paths, combat/items, ceiling impact, and the older
ledge/platform interactions. Full hops have a callback behavior test; the
supplied retail trajectory is short-hop/double-jump. Audio playback is outside
the headless simulator. No Dolphin run, protected harness/decomp modifications,
game-data additions, commits or pushes were performed by this task.
Harness tests were not run because no harness code changed.

## Files touched

- `TRACKER.md`
- `crates/hsd-particle/tests/live_fd_dash.rs`
- `crates/hsd-particle/tests/live_fd_jump.rs`
- `crates/hsd-particle/tests/support/dash_fd_spawns.rs`
- `crates/hsd-particle/tests/support/dust_replay.rs`
- `crates/hsd-particle/tests/support/jump_fd_spawns.json`
- `crates/hsd-particle/tests/support/jump_fd_spawns.rs`
- `crates/melee-ft/src/fighter/M4_JUMP.md`
- `crates/melee-ft/src/fighter/README.md`
- `crates/melee-ft/src/fighter/assets.rs`
- `crates/melee-ft/src/fighter/effects.rs`
- `crates/melee-ft/src/fighter/fall.rs`
- `crates/melee-ft/src/fighter/jump.rs`
- `crates/melee-ft/src/fighter/landing.rs`
- `crates/melee-ft/src/fighter/mod.rs`
- `crates/melee-ft/src/fighter/procs.rs`
- `crates/melee-ft/src/fighter/spawn.rs`
- `crates/melee-ft/src/fighter/state.rs`
- `crates/melee-ft/src/fighter/walk.rs`
- `crates/melee-ft/src/physics/airborne.rs`
- `crates/melee-ft/src/physics/mod.rs`
- `crates/melee-ft/tests/fighter_support/mod.rs`
- `crates/melee-ft/tests/fighter_support/replay.rs`
- `crates/melee-ft/tests/movement_fox_states.rs`
- `crates/melee-sim/src/effects.rs`
- `crates/melee-sim/src/initial_state/fighter.rs`
- `crates/melee-sim/tests/m4_gate.rs`
- `docs/PARTICLES.md`
