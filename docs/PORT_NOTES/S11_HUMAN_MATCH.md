# S11-a: recorded human match progress

Status: incomplete, stopped at the S7+S8 lane boundary. No commits or Git writes.
The enabled M5 ratchet verifies ticks 0–845 (846 ticks): all 49 canonical
fields, the item-count/12-item-field schema (62 keys with one item), and each
tick's ordered particle RNG call sites. It reads the full, unchanged 6,083-tick
recording. No expected values, tolerances, scenarios, game data or decomp files
were changed.

## First-divergence history

Ticks below are zero-based frame-end indices unless a callback stop is named.

| Tick | Key / stop | Cause and port | Retail reference |
|---|---|---|---|
| Setup | FD horizon guard | The constructor rejected the entire recording before simulation. Removed the prospective `elapsed + frames > 1800` restriction so the exact prefix can run. The runtime transition-action guard remains; live FD transition attachments are **not implemented**. | `gr/last`, stage controller and model actions |
| 329 → 330 | Stock-loss burst / RNG stream | The match-start save predates the stock HUD. Both match-start restoration and cold creation now finish the pending HUD setup from `IfAll.usd`, using its default four-slot Versus anchors, stock-model joint animation, per-icon frames and loss-animation flag. The existing Interface callback emits particle 0xF7 at the lost icon. Post-HUD savestate import remains unchanged. | `gm_Scene_Vs_OnEnter` (8016E934), `ifStatus_802F665C`, `ifStock_802F98E8`, `ifAll_802F343C`, `fn_802F9410`, `ifStock_802F8298` |
| 352 | Marth TurnRun input stop | TurnRun now invokes the existing concrete running-jump entry, reaching KneeBend. | `ftCo_TurnRun_IASA` (800C9ED8), call to `fn_800CAF78` at 800C9EE4 |
| 355 | Marth KneeBend input stop | Up-smash jump cancel uses the NoD0 predicate (no ordinary stick-age restriction), then the existing AttackHi4 entry. Earlier special-up and grab predicates retain their explicit stops. | `ftCo_KneeBend_IASA` (800CB5FC), `ftCo_AttackHi4_CheckInputNoD0` (8008C948) |
| 477 | Fox `motion_id` / `cur_anim_frame` | RebirthWait omitted aerial-attack input before aerial jump. Wired the existing aerial entry; retained the platform target X as the first shared motion-scratch word instead of rejecting this source state. Retail carries -50 (`0xC2480000`) through the transition. | `ftCo_RebirthWait_IASA` (800D575C), `ftCo_800D5600`, AttackAir entry |
| 552 | Fox Illusion ground-to-air collision stop | Added the startup/travel collision rows' appropriate air transition. Preserve animation frame and visibility, traverse command control flow without replaying owner commands, evaluate the preceding/resumed root poses in order, preserve travel hit status, spend jumps and lock the ECB. | `ftFx_SpecialSStart_GroundToAir` (800EA1D4), `ftFx_SpecialS_GroundToAir` (800EA698), `ftCommon_8007D60C`, `Fighter_ChangeMotionState` (800693AC), `ftAction_8007349C` |
| 695 | Fox `cur_pos.y`: expected `0x42CCFFFF`, actual `0x42CD0000` | Physics produced 102.49999237060547, then collision rounded it to 102.5. Revival reset the live skeleton to its rest pose before the support probe, giving a wrong locked ECB (bottom 14.047386 rather than 10.634033) and an extra collision subdivision. Retaining the outgoing live pose fixes the reset order and exact Y. | `Fighter_UnkProcessDeath` (80068354), retained-model revival/support probe |
| 768 | `items.0.motion_id`: expected 3, actual 0 | Aerial blaster pickup must initialize from the owner's current action. Added an owned held-spawn request and a pickup callback through the existing static item logic table. | `it_802AE8A8`, `Item_8026AB54`, `itFoxBlaster_Logic96_PickedUp` (802AEB00), `it_803F6E68` |
| 768 | `items.0.pos.x`: expected `0xC20F5C23`, actual `0xC20C709E` | Blaster physics incorrectly copied the moving fighter position. Retail updates attached model parts while Item.pos retains its spawn position. Removed the copy; subsequent recorded item fields match. | `itFoxblaster_UnkMotion8_Phys` (802AEED4), `it_802AE63C`, `it_802AE200` |
| **846 — stopped** | Marth KneeBend → Catch; explicit `jump cancel Grab` panic | Retail changes `p0.motion_id` 24 → 212 with `p0.cur_anim_frame = 0`. Needs the S7+S8 lane's `ftCo_Catch_CheckInput`/Catch entry wired into KneeBend in retail predicate order, with its grab scratch and callback behavior. No grab/capture files were edited. | `ftCo_KneeBend_IASA` (800CB5FC), `ftCo_Catch_CheckInput` (800D8990) |

The HUD exists before the first scheduler tick: the match-start save is inside
Versus setup, before the later HUD creation call. Both constructors complete
that same pending setup, rather than choosing a recording-specific tick.
`gm_SetupRulesDefaults` (80167A64) selects four HUD slots, independently of the two active
fighters. The new HUD test compares the constructed icons, frame bytes and loss flag
against an independent, existing post-HUD savestate.

No new motion states were needed: the existing common/family table rows now
reach the missing entries. Shared transitions use concrete `Fighter` /
`FighterCore`; Fox/Falco differences remain in the family hooks and item tables.
New tick-path requests remain in fixed buffers.

## Remaining work

Resume after the grab-lane change and fix each next first divergence. FD's
layered transitions and live model-joint attachments remain unported; the
runtime guard still fails closed when transition actions occur. The trace has
not been verified beyond tick 845, including later deaths, respawns and GAME.
Do not add the full `match_fd_foxmarth_6083_ticks_and_ordered_particle_draws`
acceptance test or its zero-allocation budget until the complete scene passes.

## Validation

| Check | Final result |
|---|---|
| Enabled human-match prefix, debug and release | `846 ticks, 62 keys, 0 divergences; ordered particle draws exact` |
| `cargo gate --no-fail-fast` | 1,104 passed, 1 failed, 3 existing ignores; only the missing decomp source oracle fails |
| `cargo gate --release --no-fail-fast` | Same: 1,104 passed, 1 failed, 3 existing ignores |
| M4 / M5 / allocation suites, included in both workspace runs | 261 / 78 / 26 passed; no failures or ignores in these suites |
| Simulator library, both profiles | 58 passed, 2 existing ignores; includes the independent stock HUD test |
| Slippi replay suite, both profiles | 9 passed |
| `tools/check-release-math.sh` | PASS, including opt levels 0, 1, 2, 3, s and z |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo fmt --all -- --check` / `git diff --check` | PASS |
| `tools/merge-check.sh lane/battlefield` | Data checks PASS; stopped because `main` is not an ancestor of the lane. No rebase attempted (Git writes prohibited). |
| `tools/perf-gate.sh` | REGRESSION in three timing runs; size and duplicate limits PASS, baseline unchanged; details below |
| Full 6,083-tick acceptance command | Incomplete: explicit jump-cancel Grab panic at tick 846 |

Final prefix output (both profiles):

```text
match_fd_foxmarth: 846 ticks, 62 keys, 0 divergences; ordered particle draws exact
test match_fd_foxmarth_exact_prefix_and_ordered_particle_draws ... ok
```

Full acceptance command:

```sh
cargo run -q --release -p melee-sim -- gate harness/scenarios/match_fd_foxmarth.toml
```

Its final error is:

```text
panicked at crates/melee-ft/src/fighter/jump.rs:204:13:
not implemented: ftCo_KneeBend.c:63-65: jump cancel Grab
```

A final `run` export to `/tmp/s11-final-actual.jsonl` contains exactly 846
completed records, ending at frame 845, before this panic.

The required `6083 ticks, 62 keys, 0 divergences` line has **not** been reached.
No full-match zero-allocation budget was added; the 26 existing budgets pass
unchanged. Logs for this session are `/tmp/s11-*.log`.

### Performance

All three measured runs pass size and duplicate limits but fail timing. Their
evidence remains in `docs/PERF.md`. The first run is
`target/perf/20260910T153053Z-28114`:

```text
[REGRESSION] perf-gate: 3637744 stripped bytes, 3325952 text bytes; load 271.749 ms; ticks_600 44.794 ms
```

The warm rerun is `target/perf/20260910T153232Z-28760`:

```text
[REGRESSION] perf-gate: 3637744 stripped bytes, 3325952 text bytes; load 257.433 ms; ticks_600 46.730 ms
```

System load averages after the second failed run were 60.778 / 45.042 / 35.404
on 12 logical CPUs. An alternating 8-runs-per-binary comparison of
`gate start_fd_fox` used the saved previous-PASS CLI from
`target/perf/20260910T140939Z-51950` and the current stripped CLI. All 16 gates
passed. Median wall/CPU times were 332.866/308.482 ms for the previous binary
and 382.091/299.665 ms for the current one. These are end-to-end CLI timings,
not replacements for Criterion's separate load and tick measurements. The
CPU measurements do not show a corresponding increase; wall timing was noisy.
Load declined to 36.987 by the end, prompting one final performance run.
A/B details are `/tmp/s11-perf-ab.log`.

The final run is `target/perf/20260910T153502Z-31874`:

```text
[REGRESSION] perf-gate: 3637744 stripped bytes, 3325952 text bytes; load 293.415 ms; ticks_600 51.009 ms
```

**Performance acceptance is not green.** Remeasure on an idle machine before
accepting this work. The A/B diagnostic suggests a shared timing disturbance;
it does not substitute for a passing perf gate or exclude all code overhead.

Duplicate labels remain 20 in melee-ft, 7 in melee-sim, and 100 across crates;
all per-character ceilings are unchanged. All 304 audited common labels have
exactly one definition. No baseline or tolerance was edited.

The lane's `third_party/melee-decomp` is an empty directory. The initial
`cargo gate` failed `dynamics_c_excerpts_match_decomp` reading
`src/melee/lb/lb_00F9.c`. Retail investigation used the existing main checkout
at `/Users/nikhilunni/Projects/melee/third_party/melee-decomp` read-only. The
protected lane directory was not modified and no missing-data opt-out was used.

## Changed files

- `crates/hsd-archive/src/desc/model.rs`: HUD scene-model descriptor reader.
- `crates/melee-sim/src/initial_state/{stock.rs,mod.rs,cold.rs,stage.rs}`: stock HUD construction, saved/cold wiring and removal of the prospective FD horizon guard.
- `crates/melee-ft/src/fighter/{dash.rs,jump.rs,landing.rs,life.rs,state/callbacks/input.rs}`: existing motion entries and retained revival scratch.
- `crates/melee-ft/src/fighter/{fall.rs,spawn.rs,commands.rs}` and `crates/ft-fox-family/src/special_s.rs`: Illusion's preserved ground-to-air transition.
- `crates/melee-ft/src/anim/playback.rs`: retain the live pose across revival reset.
- `crates/melee-it/src/{logic.rs,spawn.rs}`, `crates/it-foxlaser/src/lib.rs`, `crates/ft-fox-family/src/special_n.rs`, `crates/melee-sim/src/{scene_items.rs,frame.rs}`: held pickup dispatch and blaster initialization/position.
- `crates/melee-sim/tests/m5_gate.rs`: enabled exact 846-tick progress ratchet.
- `TRACKER.md`, this report and `docs/PERF.md`: status, handoff and measured evidence.

The independent HUD test initially caught the wrong two-slot layout:
player 0 icon 0 was `0xC16FFF00` (-14.999755859375) rather than the saved
`0xC1CFFF80` (-25.999755859375). The implementation now follows
`gm_SetupRulesDefaults` / `ifAll_802F343C(4)`; the test and its expected data
were unchanged, and the test passes.

The held request stores only the spawn descriptor. Owner inputs are sampled
when applying the request; storing a full owner snapshot in each queued entry
would enlarge every fighter and overflow `start_puff_bones_130`'s debug stack.
That intermediate implementation was corrected without raising a stack limit.
