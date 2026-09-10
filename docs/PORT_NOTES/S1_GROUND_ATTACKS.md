# S1: Fox and Marth ground attacks

Implemented for the S1/perf task in `lane/battlefield` on 2026-09-09
(recordings labelled 2026-09-10).
No commits, Git mutations, capture edits, game-data writes, item-crate changes,
or Fox special-state changes were made. The pre-existing decomp symlink is unchanged.

## Scene evidence

All nineteen scenes passed individually in debug and through the exact requested
release CLI command. All four 300-tick particle replays match every non-display
field, ordered particle draw and final seed. Workspace results are below.
The columns list attack states and target reactions in order; startup movement
and intervening Wait states are omitted. History lists successive first failures
from saved diagnostic runs, not expected values changed to obtain a pass.

| Scene | Attack states | Target states | First-divergence history | Ticks / keys / divergences |
|---|---|---|---|---|
| `dashattack_fd_fox` | 50 | 82 → 42 | unported callback/command; 85 rng.seed | 300 / 49 / 0 |
| `ftilt_fd_fox` | 53 | 79 | unported callback/command; 111 rng.seed | 300 / 49 / 0 |
| `ftiltup_fd_fox` | 51 | 79 | unported callback/command; 111 rng.seed | 300 / 49 / 0 |
| `ftiltdown_fd_fox` | 55 | 82 | unported callback/command; 112 rng.seed | 300 / 49 / 0 |
| `dtilt_fd_fox` | 57 | 83 → 42 | unported callback/command | 300 / 49 / 0 |
| `fsmashcharge_fd_fox` | 60 | 77 → 42 | unported callback/command; effect dispatch | 300 / 49 / 0 |
| `usmash_fd_fox` | 63 | 90 → 183 → 184 | unported callback/command; 113 p1.motion_id | 300 / 49 / 0 |
| `dsmash_fd_fox` | 64 | 82 → 42 | unported callback/command; 106 rng.seed; 112 rng.seed | 300 / 49 / 0 |
| `jabcombo_fd_fox` | 44 → 45 → 47 → 48 → 49 | 78 → 76 → 42 → 76 → 42 → 76 → 42 → 76 → 42 → 76 → 42 → 76 → 42 | unported callback/command; 140 p1.kb_vel.x; 168 p0.cur_pos.x; 181 p0.cur_anim_frame | 300 / 49 / 0 |
| `dashattack_fd_marth` | 50 | 79 → 42 | Loader rejected `Mars`; unported callback/command; effect dispatch | 300 / 49 / 0 |
| `ftilt_fd_marth` | 53 | 79 → 42 | Loader rejected `Mars`; unported callback/command; effect dispatch | 300 / 49 / 0 |
| `ftiltup_fd_marth` | 53 | 79 → 42 | Loader rejected `Mars`; unported callback/command; effect dispatch | 300 / 49 / 0 |
| `ftiltdown_fd_marth` | 53 | 79 → 42 | Loader rejected `Mars`; unported callback/command; effect dispatch | 300 / 49 / 0 |
| `dtilt_fd_marth` | 57 | 79 → 42 | Loader rejected `Mars` | 300 / 49 / 0 |
| `fsmashcharge_fd_marth` | 60 | 87 → 38 → 0 | Loader rejected `Mars`; unported callback/command; 191 p1.motion_id; 176 p1.cur_anim_frame; 230 p0.cur_anim_frame; 230 rng.seed | 300 / 49 / 0 |
| `usmash_fd_marth` | 63 | 88 → 38 → 0 | Loader rejected `Mars`; unported callback/command; effect dispatch; 168 p1.cur_anim_frame; 221 p0.cur_anim_frame; 221 rng.seed | 300 / 49 / 0 |
| `dsmash_fd_marth` | 64 | 89 → 252 → 253 | Loader rejected `Mars`; 108 rng.seed; unported callback/command; 158 p1.cur_anim_frame; 162 p1.cur_anim_frame | 300 / 49 / 0 |
| `jabcombo_fd_marth` | 44 → 45 | 79 | Loader rejected `Mars`; unported callback/command | 300 / 49 / 0 |
| `utilt_fd_fox` | 56 | Wait (no hit) | Already exact | 300 / 49 / 0 |

Each successful CLI gate prints exactly `300 ticks, 49 keys, 0 divergences`.
`m5_gate.rs` includes all nineteen scenes, compares all 49 fields including RNG
seed on every tick, and checks the ordered particle call-site sequence against
its independent ledger. Existing eight combat gates are retained.

The nineteen independent ledgers exercise these non-particle-interpreter sites:

| Retail sites | Purpose | Draws across nineteen scenes |
|---|---|---:|
| 8009FCDC / 8009FD00 / 8009FD24 | Graphics offset X/Y/Z, including charge sparkle | 164 each |
| 80063990 | Small normal spark selection | 18 |
| 800785CC | Optional normal spark selection | 9 |
| 80063B70 | Slash spark orientation | 8 |
| 80088A18 | Character smash sound selection | 6 |
| 802F4D44 / 802F4D54 | Percent digit shake X/Y | 896 each |
| 802F496C / 802F499C | KO percent digit velocity X/Y | 8 each |
| 8008A8BC | Wait animation choice | 37 |
| 8021AFFC / 8021AEC8 | Existing FD stage setup/resume | 19 each |

The effect and sound paths above preserve their caller order around generator
creation. No extra damage-voice RNG site is present in these ledgers.

## Ported behavior

- Static common-state rows cover dash attack, all side-tilt angle variants,
  down tilt, up/down smash, jab continuation and rapid-jab start/loop/end.
  Shared callbacks remain in attack, smash, damage and down modules.
- Motion availability and archive angle thresholds select Fox's angled tilts;
  missing Marth variants fall back to AttackS3S. The selector lives on the
  concrete `FighterCore`. No fighter-kind branch was added.
  `CharacterCallbacks::third_jab_state` preserves Marth's two-hit chain; rapid jab
  requires its archive-enabled script flag and attribute threshold.
- Every hitbox, including Marth tipper geometry, comes from the shared typed
  subaction decoder and melee-coll. Commands 27/28 set all/bone hurt-capsule status;
  29 enables jab continuation; 30 enables rapid jab. Throw flag zero also marks
  the rapid-jab loop checkpoint (ftAction_800718A4).
- Charge overlays are decoded once from PlCo color-animation scripts into Color,
  Graphics, Wait and Goto instructions. The fixed playback cursor produces the
  charge sparkle commands and their three offset RNG draws in retail order.
  Hold/release timing, maximum charge and the charge sound use archive parameters.
- Damage state selection covers grounded height/strength variants and tumble,
  including DamageFlyTop's angle window. Tumble smoke uses retail speed thresholds,
  the shared particle interpreter, and the ordinary DamageFall/down/ledge paths.
- Normal hit effects include large sparks and the conditional extra spark draw
  (800785CC/800785FC), while slash sparks retain their rotation draw (80063B70).
  The common effect table and prepared model pool cover the new dust/smash models.
  Particle replay caught the missing integer conversion for hit-spark scale
  (Fox dsmash tick 112, usmash tick 113, charged fsmash tick 146); the production
  conversion was fixed and fixtures regenerated through the gate.
- A fixed ten-entry stale history registers attack instances once, including
  rapid-jab animation wraps. Knockback retains the pre-stale integer damage count
  from ftColl_8007ABD0; percent uses staled damage. The repeated-hit separation
  routine ftColl_80076528 runs after physics even during hitlag; its two fnmsubs
  sites (8007658C/800765A0) were checked in retail assembly.
- Final-stock elimination uses gm_GetFFAOutcome (8016BF74) and the next-tick
  gm_803DA888[4] pause mask (0x800FFA), indexed by object group. Gameplay and
  ordinary particle/effect owners pause; the KO effect and auxiliary particles
  continue. This differs from the existing non-final-stock KO/revival scene.
- The scenario loader accepts the recording's retail `Mars` spelling as the
  existing `Marth` configuration name. Scenario files are unchanged.

## Previously unidentified target states

183/184 are `DownBoundU` / `DownWaitU`, the face-up knockdown and prone wait.
The existing face-down behavior is shared; ftCo_80097570 selects the orientation
from HipN's animated matrix before landing. Fox up smash exercises the face-up rows.

252/253 are `CliffCatch` / `CliffWait`, already implemented ledge states.
Marth down smash exercises them when hitstun expires into DamageFall and the
collision pass catches the ledge. Tumble collision itself must not catch early.

## Particle and allocation evidence

Four full 300-tick fixture replays were added: `fsmashcharge_fd_fox`,
`usmash_fd_fox`, `dsmash_fd_fox`, and `dsmash_fd_marth`. Inputs were exported by
`fixture-spawns`, which gates the complete scenario before writing. They contain
external requests/matrices only, never particle outputs or injected RNG seeds.
The fixture reader now accepts the precisely identified extra-spark Randi events
with their bounds, checking their retail RNG entry point and ordered call site.

A prepared 256-slot AppSRT owner pool replaces repeated Arc allocations on spawn
and attachment updates. Slots are reused only after all live aliases expire;
pool exhaustion is explicit. When a diagnostic caller retains an Arc beyond its
last simulated owner, the pool transfers that allocation to the caller, preserving
the existing ownership contract. Both existing AppSRT lifetime regressions pass
unchanged. Existing allocation ceilings remain 17/1/12/690/17.
Particle vectors and the diagnostic draw buffer are also reserved at simulation
construction. The effect runtime has unique boxed ownership allocated at setup,
keeping cold-start state moves within the default test stack after combat storage
grew; the previously overflowing Battlefield cold-start test passes unchanged.
Both new allocation ceilings are zero: Fox jab combo and charged
forward smash allocate nothing during their 299 measured ticks. Existing start,
idle and Marth jab scenes now measure zero; KO measures 669 (previously 690),
with the existing revival peak of 668 and one stock-HUD allocation.

## Final validation

Completed across 2026-09-09/10.

The nineteen commands `cargo run -q --release -p melee-sim -- gate
harness/scenarios/<scene>.toml` all exited 0, each printing exactly:

```text
300 ticks, 49 keys, 0 divergences
```

- `cargo gate`: 1,003 passed, 0 failed, 3 existing ignored (aggregate of Cargo
  test binaries and doctests), rechecked after the final concrete-core helper
  move. M4: 261 passed; M5: 27 passed.
- `cargo gate --release`: 1,003 passed, 0 failed, the same 3 existing ignored.
  M4: 261 passed; M5: 27 passed. The original 977 tests remain, with 19 new
  scene gates, 4 particle replays, 2 allocation gates and 1 command-decode test.
- `tools/check-release-math.sh`: passed native math tests in both profiles
  and signed-zero/fused tests at opt levels 0, 1, 2, 3, s and z (74 tests total).
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `cargo build --workspace --all-targets --locked`: passed.
- Separate `cargo test -p melee-sim --test m4_gate`: passed, exact final line:

```text
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 146.68s
```

The four particle replay final lines are:

```text
dsmash_fd_fox matched 300/300 ticks: 522645 fields, 10058 ordered particle draws, all final seeds; final seed 0x7fc33684
dsmash_fd_marth matched 300/300 ticks: 475799 fields, 9306 ordered particle draws, all final seeds; final seed 0x03fc2606
fsmashcharge_fd_fox matched 300/300 ticks: 533399 fields, 10168 ordered particle draws, all final seeds; final seed 0x2101568f
usmash_fd_fox matched 300/300 ticks: 521916 fields, 10003 ordered particle draws, all final seeds; final seed 0x59eadbeb
```

Every replay reports `mismatches: 0`. The existing AppSRT display-cache
exclusion remains unchanged (2,100 / 1,036 / 3,808 / 2,100 fields respectively).
Both new allocation tests report `simulate-only 0 (0.000000/tick), peak 0,
allocating ticks 0` across 299 measured ticks; owned diagnostic snapshots are
counted separately, under the existing test contract.

`tools/merge-check.sh lane/battlefield` exited 1 at its ancestry prerequisite:

```text
[PASS] data: no tracked game data or protected lane changes
[PASS] data: oracle traces present
[FAIL] rebase: main is not an ancestor of lane/battlefield (or ref lookup failed)
```

Read-only Git checks confirmed the ancestry mismatch: lane HEAD `cd6cfad`,
main `bcfc727`. The second invocation disabled the sandbox-inaccessible Git
fsmonitor through process environment only; it produced the same ancestry
failure without the unrelated IPC diagnostic. No rebase was attempted because
the task prohibits Git write commands. The build and test checks above were
run independently; a merge-check pass on a rebased tree remains for integration.

## Scope limits

This ports the branches exercised by the required scenes, not every unrecorded
combat or match-flow branch. Existing explicit boundaries remain for unsupported
DI/SDI inputs, airborne follow-up hits, non-neutral recovery choices and other
blast directions. No existing test was weakened or expected value edited.

## Changed files

- `TRACKER.md`
- `crates/ft-fox/src/init.rs`
- `crates/ft-mars/src/init.rs`
- `crates/hsd-particle/src/system.rs`
- `crates/hsd-particle/tests/data/README.md`
- `crates/hsd-particle/tests/data/dsmash_fd_fox_spawns.json`
- `crates/hsd-particle/tests/data/dsmash_fd_marth_spawns.json`
- `crates/hsd-particle/tests/data/fsmashcharge_fd_fox_spawns.json`
- `crates/hsd-particle/tests/data/usmash_fd_fox_spawns.json`
- `crates/hsd-particle/tests/live_dsmash_fd_fox.rs`
- `crates/hsd-particle/tests/live_dsmash_fd_marth.rs`
- `crates/hsd-particle/tests/live_fsmashcharge_fd_fox.rs`
- `crates/hsd-particle/tests/live_usmash_fd_fox.rs`
- `crates/hsd-particle/tests/support/dust_replay.rs`
- `crates/hsd-particle/tests/support/fixture_spawns.rs`
- `crates/melee-cmd/src/decode.rs`
- `crates/melee-cmd/src/lib.rs`
- `crates/melee-cmd/tests/interpreter.rs`
- `crates/melee-coll/src/damage.rs`
- `crates/melee-coll/src/detection.rs`
- `crates/melee-coll/src/hitbox.rs`
- `crates/melee-ef/src/dust.rs`
- `crates/melee-ef/src/fixture_spawns.rs`
- `crates/melee-ef/src/lib.rs`
- `crates/melee-ef/src/pool.rs`
- `crates/melee-ef/src/request.rs`
- `crates/melee-ef/src/tables.rs`
- `crates/melee-ft/src/fighter/assets.rs`
- `crates/melee-ft/src/fighter/attack.rs`
- `crates/melee-ft/src/fighter/attack/combo.rs`
- `crates/melee-ft/src/fighter/attack/stale.rs`
- `crates/melee-ft/src/fighter/caches.rs`
- `crates/melee-ft/src/fighter/commands.rs`
- `crates/melee-ft/src/fighter/damage.rs`
- `crates/melee-ft/src/fighter/dash.rs`
- `crates/melee-ft/src/fighter/down.rs`
- `crates/melee-ft/src/fighter/effects.rs`
- `crates/melee-ft/src/fighter/mod.rs`
- `crates/melee-ft/src/fighter/procs.rs`
- `crates/melee-ft/src/fighter/run.rs`
- `crates/melee-ft/src/fighter/smash.rs`
- `crates/melee-ft/src/fighter/spawn.rs`
- `crates/melee-ft/src/fighter/state/callbacks/animation.rs`
- `crates/melee-ft/src/fighter/state/callbacks/input.rs`
- `crates/melee-ft/src/fighter/state/callbacks/physics.rs`
- `crates/melee-ft/src/fighter/state/common_table.rs`
- `crates/melee-sim/src/frame.rs`
- `crates/melee-sim/src/initial_state/cold.rs`
- `crates/melee-sim/src/initial_state/mod.rs`
- `crates/melee-sim/src/scenario.rs`
- `crates/melee-sim/tests/alloc_gate.rs`
- `crates/melee-sim/tests/m5_gate.rs`
- `docs/PORT_NOTES/S1_GROUND_ATTACKS.md`
