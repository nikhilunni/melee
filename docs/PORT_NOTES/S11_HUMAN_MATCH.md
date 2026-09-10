# S11-a: recorded human match progress

Status: both full recordings, M4/M5/allocation suites and performance pass on the combined S7+S8 tree (2026-09-10 continuation). Workspace/merge acceptance has the two checkout blockers below. No commits or Git writes.
The first match passes all 6,083 ticks, including item fields and ordered particle draws. Its simulate-only allocation budget is zero and passes. The second match also passes all 10,059 ticks, its full ordered-draw test, and a zero allocation budget. No expected values, tolerances, scenarios, game data or decomp files were changed.

Full-match focused results:

```text
6083 ticks, 62 keys, 0 divergences
10059 ticks, 62 keys, 0 divergences
match_fd_foxmarth: 6082 measured ticks; simulate-only 0 (0.000000/tick), peak 0, allocating ticks 0; with snapshot 760250 (125.000000/tick), peak 125; snapshot overhead 760250 (125.000000/tick)
match2_fd_foxmarth: 10058 measured ticks; simulate-only 0 (0.000000/tick), peak 0, allocating ticks 0; with snapshot 1257250 (125.000000/tick), peak 125; snapshot overhead 1257250 (125.000000/tick)
```

## First-divergence history

Ticks below are zero-based frame-end indices unless a callback stop is named.
| Tick | Key / stop | Cause and port | Retail reference |
|---|---|---|---|
| Setup | FD horizon guard | The constructor rejected the entire recording before simulation. Removed the prospective `elapsed + frames > 1800` restriction so the exact prefix can run. Live FD layer and tilt attachments are implemented; later material actions beyond these recordings retain explicit stops. | `gr/last`, stage controller and model actions |
| 329 → 330 | Stock-loss burst / RNG stream | The match-start save predates the stock HUD. Both match-start restoration and cold creation now finish the pending HUD setup from `IfAll.usd`, using its default four-slot Versus anchors, stock-model joint animation, per-icon frames and loss-animation flag. The existing Interface callback emits particle 0xF7 at the lost icon. Post-HUD savestate import remains unchanged. | `gm_Scene_Vs_OnEnter` (8016E934), `ifStatus_802F665C`, `ifStock_802F98E8`, `ifAll_802F343C`, `fn_802F9410`, `ifStock_802F8298` |
| 352 | Marth TurnRun input stop | TurnRun now invokes the existing concrete running-jump entry, reaching KneeBend. | `ftCo_TurnRun_IASA` (800C9ED8), call to `fn_800CAF78` at 800C9EE4 |
| 355 | Marth KneeBend input stop | Up-smash jump cancel uses the NoD0 predicate (no ordinary stick-age restriction), then the existing AttackHi4 entry. At this point the earlier special-up and grab predicates retained explicit stops; grab is ported at tick 846 below. | `ftCo_KneeBend_IASA` (800CB5FC), `ftCo_AttackHi4_CheckInputNoD0` (8008C948) |
| 477 | Fox `motion_id` / `cur_anim_frame` | RebirthWait omitted aerial-attack input before aerial jump. Wired the existing aerial entry; retained the platform target X as the first shared motion-scratch word instead of rejecting this source state. Retail carries -50 (`0xC2480000`) through the transition. | `ftCo_RebirthWait_IASA` (800D575C), `ftCo_800D5600`, AttackAir entry |
| 552 | Fox Illusion ground-to-air collision stop | Added the startup/travel collision rows' appropriate air transition. Preserve animation frame and visibility, traverse command control flow without replaying owner commands, evaluate the preceding/resumed root poses in order, preserve travel hit status, spend jumps and lock the ECB. | `ftFx_SpecialSStart_GroundToAir` (800EA1D4), `ftFx_SpecialS_GroundToAir` (800EA698), `ftCommon_8007D60C`, `Fighter_ChangeMotionState` (800693AC), `ftAction_8007349C` |
| 695 | Fox `cur_pos.y`: expected `0x42CCFFFF`, actual `0x42CD0000` | Physics produced 102.49999237060547, then collision rounded it to 102.5. Revival reset the live skeleton to its rest pose before the support probe, giving a wrong locked ECB (bottom 14.047386 rather than 10.634033) and an extra collision subdivision. Retaining the outgoing live pose fixes the reset order and exact Y. | `Fighter_UnkProcessDeath` (80068354), retained-model revival/support probe |
| 768 | `items.0.motion_id`: expected 3, actual 0 | Aerial blaster pickup must initialize from the owner's current action. Added an owned held-spawn request and a pickup callback through the existing static item logic table. | `it_802AE8A8`, `Item_8026AB54`, `itFoxBlaster_Logic96_PickedUp` (802AEB00), `it_803F6E68` |
| 768 | `items.0.pos.x`: expected `0xC20F5C23`, actual `0xC20C709E` | Blaster physics incorrectly copied the moving fighter position. Retail updates attached model parts while Item.pos retains its spawn position. Removed the copy; subsequent recorded item fields match. | `itFoxblaster_UnkMotion8_Phys` (802AEED4), `it_802AE63C`, `it_802AE200` |
| 846 | Marth KneeBend → Catch | Wired the existing Catch entry ahead of up-smash, preserving retail predicate order. Reviewed the merged remapped `ThrowSource` and resumed-pose motion entry. | `ftCo_KneeBend_IASA` (800CB5FC), `ftCo_Catch_CheckInput` (800D8990) |
| 919 | Fox `cur_pos.y`: expected `0x4073C824`, actual `0x4141BED7` | First forward throw after respawn used zero capture geometry. Preserve the costume geometry across resets and compute it from XRotN/TransN for cold creation. | `Fighter_UnkInitReset` (80067C98), `Fighter_UnkUpdateVecFromBones_8006876C` |
| 936 | Hitstun jump-input stop | Store remaining hitstun at jump input; consume a valid buffer in Damage animation/IASA after hitstun. | `doIasa` (8008F938), `ftCo_Damage_Anim` (8008F7F0), `ftCo_Damage_IASA` (8008FA44) |
| 1001 | Marth dash-grab stop | Added CatchDash/CatchDashPull table rows, animation resources, running grab predicates, root-motion physics and linked capture selection. | `ftCo_800D8A38`, `ftCo_800D8C54`, `ftCo_CatchDash_Phys` (800D8DD0), `fn_800D9CE8` |
| 1065 | Fox `cur_pos.x`: expected `0xC1AA452A`, actual `0xC1AA478D` | Repeated forward throw lacked staling (4 damage versus 3.68); throw knockback also requires the original damage count. Added throw/pummel move identities and throw-history recording. | `ftColl_8007891C`, `plStale_UpdateStaleMovesFromFighter`, `ftAction_80071F0C`, `ftColl_80079AB0` |
| 1252 | Fox `facing_dir`: expected +1, actual -1 | Side-special entry omitted the signed stick reversal threshold. | `ftCo_SpecialAir_CheckInput`, `ftCo_SpecialS_CheckInput`, `ftCommon_UpdateFacing` |
| 1381 | Fox `kb_vel.x`: expected `0x3E80B28C`, actual `0x3EB07675` | A grounded hit after an earlier launch replaced knockback instead of combining opposite components after PlCo +FC. | `ftCo_Damage_CalcVel` (8008DC0C) |
| 1495 | Fixed sound-output queue exhausted (64 entries) | Headless simulation retained every sound request across frames. Retire sound and rumble requests at the next frame boundary; keep fixed capacities. | `ftAction_80071B50`, `ftAction_800728F8`; scene output lifecycle |
| 1581 | Marth `cur_anim_frame`: expected 1, actual 16 | Fox's nair was skipped because a previous throw-owner exception survived ordinary motion changes. Clear it at motion entry and restore the release exception in the damage-entry callback order. | `Fighter_ChangeMotionState` (800693AC), `ftCo_800DE7C0`, `fn_800DE798`, `ftColl_8007B8CC` |
| 1605 | Fox dash side-special input stop | Enter side special in the early/redash input window and apply the shared successful-interrupt friction tail. | `ftCo_Dash_IASA` (800CA230), fmadds at 800CA51C |
| 1886 | FD transition action stop | Release the stage start flag with countdown GO, preload layer animation tracks, apply single-joint/subtree switches without replacing joint identities, and remove old attached generators. Feed live joint clocks into the phase machine and attachments. Both constructors use the same prepared models; idle import restores saved joint clocks. | `Ground_801C0FB8`, `grLast_8021A9AC`, `grLast_8021B920`, `grAnime_801C7FF8`, `grAnime_801C8098`, `grAnime_801C7980`, `grAnime_801C7A94` |
| 1941 | Fox `self_vel.x`: expected -0.006999969, actual 0 | Counter contact omitted the attacker's shared shield recoil. Retained Marth lightshield and integer hit damage produce the residual after hitlag and friction. | `ftColl_80076CBC`, `Fighter_ProcessHit` (8006D8D8), `ftCommon_8007E2A4` |
| 2265 | `items.count`: expected 0, actual 1 | Laser crossed FD's right blast zone. Added item bounds retirement between velocity integration and environmental movement; attached articles skip it. | `Item_802697D4`, `Item_802696CC` |
| 3037 | Marth Run grab input stop | Run's grab predicate now enters the existing CatchDash path. | `ftCo_Run_IASA` (800CA830), `ftCo_800D8A38` |
| 3057 | Throw DI stop | Apply existing shared DI immediately after throw damage entry, without SDI/ASDI or hitlag. | `ftCo_800DE7C0`, `ftCo_8008E5A4` |
| 3378 | Ordinary airborne hit stop | Enable air reaction-table selection and launch handling; port airborne shield timing reduction for the applicable motion states. | `ftCo_8008DCE0`, `ftCo_Damage_CheckAirMotion` (8008E498) |
| 3677 | `items.count`: expected 0, actual 1 | Fox's blaster survived the damage proc. Character take-damage hook now queues synchronous blaster removal before damage entry. | `ftCommon_8007DB58`, `ftFx_SpecialN_RemoveBlaster` (800E5EBC), `it_802AEAB4` |
| 4525 | Marth `cur_anim_frame`: expected 1, actual 4 | Initial dash's forward-smash predicate incorrectly used the ordinary stick-age check. Added its facing-based predicate and the shared interrupt-friction tail. | `ftCo_AttackS4_8008C114`, `ftCo_Dash_IASA` (800CA230) |
| 4744 | Graphics request 0x513 stop | Route quake 2 through the existing randomized-offset graphics request and camera output queue. | `ftCo_8009F7F8`, `efAsync_Spawn` kind 8, `Camera_RequestQuake` |
| 4930 | Airborne Damage special-input stop | Dispatch the existing airborne special entry once hitstun ends. | `ftCo_Damage_IASA`, `ftCo_SpecialAir_CheckInput` |
| 4930 | Aerial Shield Breaker entry stop, then `p0.cur_pos.x` | Added the four aerial rows, asset preload, air entry/charge/release, no-input gravity/friction and retained knockback decay. | `ftMs_SpecialAirN_Enter`, `ftMs_SpecialAirNStart_Phys`, `ft_80084EEC`, `Fighter_procUpdate` |
| 4950 | `rng.seed`: expected `0x06F70010`, actual `0xFA91038C` | Landing during Shield Breaker release destroyed its continuing effect. Added typed motion-preservation options for effects, hitboxes and hit status, matching the phase's retail flags. | `ftMs_SpecialN_801372A8`, `Fighter_ChangeMotionState` KeepGfx/SkipHit |

### Second match
| Tick | Key / stop | Cause and port | Retail reference |
|---|---|---|---|
| 512 | Marth `cur_anim_frame`: expected 1, actual 0 | Jab 2 looping to jab 1 used ordinary follow-up entry, omitting the extra animation step and jab-2 buffer reset. | `doAttack13` → `doAttack12Rapid` → `checkAttack11`, `ftCo_Attack1.c` |
| 1278 | Fox `cur_pos.x`: expected -86.570343, actual -86.816528 | Catch collision was calling leave-ground for an already airborne fighter, resetting the ECB lock after DownBound and producing a later wall push. The shared Fall entry already performs the conditional conversion. | `ftCo_Catch_Coll`, `ftCo_Fall_Enter` |
| 2535 | Electric HitSpark stop, then `rng.seed` | Added electric impact generator 0xC and the electric damage color script, including loop/subroutine decoding and live-joint generator 0x13. The primary color script runs at the zero-frame damage restart. | `ftColl_8007A06C`, `efAsync_Dispatch` 0x3E9/0x412, `ftCo_8008DA4C`, `ftCo_800C0408`, `Command_03/04/05/06` |
| 2539 | Marth `cur_anim_frame`: expected 1, actual 2 | Electric-hit victims use PlCo +1A4 to multiply the integer hitlag before crouch scaling. | `ftColl_8007A06C`, `ftCommon_CalcHitlag` (8007DACC) |
| 2558 | `rng.seed`: expected 0x69C8913D, actual 0x9C7C9E0D | The primary damage color overlay survives ordinary motion entry and advances during zero-frame command restart. Only the secondary overlay is reset by ordinary motion changes. | `ftCo_800C0134`, `Fighter_ChangeMotionState`, `ftCo_800C0408` |
| 2811 | Fox `ground_or_air`: expected 1, actual 0 | Strong downward grounded hits must launch and reflect vertical knockback off the floor above PlCo +1E8, scaled by +1EC. Added the impact effect and suppressed the ordinary queued launch voice on that branch. | `ftCo_8008DCE0` blocks 21..28, `lbVector_Angle`, `efAsync_Spawn` 0x406 |
| 2833 | Neutral-tech entry stop | Added Passive and PassiveStandF rows, neutral ground physics, shared residual knockback projection and tech color request. | `ftCo_800987D0`, `ftCo_800989D4`, `ftCommon_8007CCE8` |
| 2833 | Ordered particle draws, same final RNG seed | Tech flash was resolved before the landing animation's dust. Deferred direct effect requests now retain their retail order after the pending animation graphics. Fixed buffers insert without allocation. | `ftCo_800987D0`, `Fighter_ChangeMotionState`, `efSync_Spawn` |
| 3075 | Marth `percent`: expected 43.129997, actual 43.369999 | Projectiles omitted their owner's stale history. Added shared typed move/instance data, projectile source capture, item-hit staling and history registration; blaster firing cycles renew the attack instance. | `plStale_UpdateStaleMovesFromItem`, `ftFx_SpecialN_OnChangeAction`, `ft_800892A0` |
| 3147 | Marth `cur_anim_frame`: expected 0, actual 7 | Grounded damage sliding backward over a ledge must enter MissFoot. Added its common row, air drift clamp, fall physics/collision and transition to DamageFall. | `ft_800848DC`, `ftCo_8009F39C`, `ftCo_MissFoot_*` |
| 4850 | Fox `facing_dir`: expected -1, actual +1 | Aerial neutral special reverses from the buffered horizontal crossing direction within PlCo +224. Added this predicate and the air-special priority order. | `ftCo_SpecialAir_CheckInput` |
| 4949 | `rng.seed`: missing protected contact | Respawn invincibility was conflated with ledge intangibility. Restore and age separate timers; invincible fighter contacts emit the impact flash and attacker hitlag without damage or staling. | `ftColl_80076CBC`, `ftCo_Rebirth`, Fighter +1990/+1994 |
| 5023 | `p0.kb_vel.x`: expected -0.24034794, actual -0.19297332 | PlCo +FC is an integer knockback-replacement window; reading it as f32 selected the combine branch too early. | `ftCo_Damage_CalcVel` (8008DC0C) |
| 7095 | FD tilt generator creation stop | Create bank 30 generator 30001, share its AppSRT with children, update origin and Euler orientation from the live tilted model, and support explicit generator retirement. | `grLast_8021B920` case 9, `grLib_801C96F8`, `grLast_8021ADD0`, `lbVector_EulerAnglesFromPartialONB`, `hsd_8039D4DC` |
| 7539 | Aerial Dancing Blade entry stop | Added all nine aerial rows and preloaded motions, once-per-airtime entry boost, phase-specific air physics, continuation and retained ground/air animation transitions. | `ftMs_SpecialAirS_Enter`, `ftMs_SpecialAirS1_*`, `ftMs_SpecialS2/S3/S4_*` |
| 8901 | `rng.seed`: expected 0xA2F6A9BA, actual 0x99860995 | Tilt completion reads map 7 joint 2's animation, while the generator attaches to joint 5. Corrected the clock used to enter Flash and retire the emitter; subsequent quake phase runs through the existing action table. | `grLast_8021B5C4`, `grNLa_803E8010`, `grLast_8021B920` cases 10/11 |
| 9867 | `p0.cur_anim_frame`: expected 35, actual 0 | DamageFly IASA delegates to DamageFall and does not synthesize the stored jump at hitstun end. Its animation callback consumes the buffer only when the animation ends. | `ftCo_DamageFly_IASA`, `ftCo_DamageFly_Anim`, `inlineC0` |
| 9880 | `p0.cur_anim_frame`: expected 0, actual 48 | Fresh horizontal input exits tumble through DamageFall IASA even while the current row remains DamageFlyTop. | `ftCo_DamageFall_IASA` |

The HUD exists before the first scheduler tick: the match-start save is inside
Versus setup, before the later HUD creation call. Both constructors complete
that same pending setup, rather than choosing a recording-specific tick.
`gm_SetupRulesDefaults` (80167A64) selects four HUD slots, independently of the two active
fighters. The new HUD test compares the constructed icons, frame bytes and loss flag
against an independent, existing post-HUD savestate.

CatchDash and CatchDashPull are new common-table rows; other existing rows now
reach missing entries. Shared transitions use concrete `Fighter` /
`FighterCore`; Fox/Falco differences remain in the family hooks and item tables.
New tick-path requests remain in fixed buffers.

## Final validation

No divergence remains in either recording. The final commands are:

```sh
cargo run -q --release -p melee-sim -- gate harness/scenarios/match_fd_foxmarth.toml
cargo run -q --release -p melee-sim -- gate harness/scenarios/match2_fd_foxmarth.toml
```

Their complete final lines are `6083 ticks, 62 keys, 0 divergences` and
`10059 ticks, 62 keys, 0 divergences`, respectively.

| Check | Final result |
|---|---|
| M4, debug and release | 261 passed, 0 failed, 0 ignored |
| M5, debug and release | 89 passed, 0 failed, 0 ignored; both full human recordings include item keys and ordered particle draws |
| Allocation suite, debug and release | 33 passed, 0 failed, 0 ignored; both human matches have strict zero simulate-only budgets |
| Simulator library, debug and release | 59 passed, 0 failed, 2 existing ignores; includes cold setup, independent stock HUD and FD attachment checks |
| Slippi suite, debug and release | 9 passed, 0 failed, 0 ignored |
| `cargo gate --no-fail-fast` | 1,125 passed, 1 failed, 3 existing ignores; missing lane-local decomp C source only |
| `cargo gate --release --no-fail-fast` | Same: 1,125 passed, 1 failed, 3 existing ignores |
| `tools/check-release-math.sh` | PASS: native math oracles in both profiles and fused tests at opt levels 0, 1, 2, 3, s, z |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo fmt --all -- --check`, `git diff --check` | PASS |
| `tools/merge-check.sh lane/battlefield` | Data checks PASS; ancestry check blocked, exact output below |
| `tools/perf-gate.sh` | PASS on final rerun; all measurements and duplicate-census explanation below |

Both M5 runs include these final lines:

```text
test match_fd_foxmarth_6083_ticks_and_ordered_particle_draws ... ok
test match2_fd_foxmarth_10059_ticks_and_ordered_particle_draws ... ok
```

The full allocation lines for both profiles are at the top of this report.
The FD attachment test independently compares one captured joint matrix and six
AppSRT transforms across the layer/tilt/Flash phases, exactly:

```text
FD transitions: 1 attached matrices and 6 AppSRT transforms exact
```

The first debug workspace run exposed cold-constructor stack overflow in
`cold_run_reads_only_dat_assets` and `start_dl_fox_cold_600`. The roster macro
expanded seven large concrete `Fighter` results in one opt-level-0 stack frame.
Character-payload selection now has its own small constructor; the concrete
fighter constructor is called once after selection. Both full suites now pass
with the ordinary test-thread stack. No stack limit or test was changed.

The lane's `third_party/melee-decomp` is empty. Both full workspace commands end:

```text
error: 1 target failed:
    `-p melee-lb --test dynamics_ref_oracle`
```

`dynamics_c_excerpts_match_decomp` fails at
`crates/melee-lb/tests/dynamics_ref_oracle.rs:168:59` while reading the missing
`third_party/melee-decomp/src/melee/lb/lb_00F9.c` (`No such file or directory`).
A supplemental read-only check of the same excerpt assertions against
`/Users/nikhilunni/Projects/melee/third_party/melee-decomp` passes. This does not
replace the failed workspace command. No protected directory or missing-data
opt-out was used.

The final merge check ends:

```text
[PASS] data: no tracked game data or protected lane changes
[PASS] data: oracle traces present
[FAIL] rebase: main is not an ancestor of lane/battlefield (or ref lookup failed)
```

These are the remaining acceptance blockers. Repairing the shared decomp path
and lane ancestry requires workspace/Git changes outside this task's allowed
scope. No Git writes were attempted. Final logs are
`/tmp/s11b-{accept-first-final,accept-second-final,gate-debug-confirmed,gate-release-final,math-final,clippy-confirmed,fmt-final,merge-check-confirmed}.log`.

## Previous session performance

Before the reviewer rebase, three loaded-machine timing runs failed while size
and duplicate checks passed. Their final lines were:

```text
[REGRESSION] perf-gate: 3637744 stripped bytes, 3325952 text bytes; load 271.749 ms; ticks_600 44.794 ms
[REGRESSION] perf-gate: 3637744 stripped bytes, 3325952 text bytes; load 257.433 ms; ticks_600 46.730 ms
[REGRESSION] perf-gate: 3637744 stripped bytes, 3325952 text bytes; load 293.415 ms; ticks_600 51.009 ms
```

The recorded system load was 60.778 / 45.042 / 35.404 on 12 logical CPUs.
Those runs do not establish current performance. Their prior-run artifacts are
`target/perf/20260910T153053Z-28114`,
`20260910T153232Z-28760`, `20260910T153502Z-31874`.
No baseline or tolerance was edited.

## Current performance

No baseline, ceiling or tolerance was changed. The first run met timing and
size limits, but found one extra cross-crate label: the generic empty
`CharacterCallbacks::take_damage` default was emitted in five character crates.
The hook is now optional static data (`TAKE_DAMAGE`), with concrete callbacks
only for characters that need it.

```text
target/perf/20260910T174208Z-72392
[REGRESSION] perf-gate: 3672832 stripped bytes, 3358720 text bytes; load 168.173 ms; ticks_600 24.718 ms
```

The first run after that correction met size and duplicate limits but failed
timing, with two high outliers. The final run used the same source while other
work in this turn was idle; system load was 7.40 / 15.18 / 14.67 before it and
7.20 / 14.04 / 14.28 afterward, on 12 logical CPUs. Both measurements are retained:

```text
target/perf/20260910T174410Z-73024
[REGRESSION] perf-gate: 3672736 stripped bytes, 3358720 text bytes; load 172.846 ms; ticks_600 27.511 ms

target/perf/20260910T174525Z-73600
[PASS] perf-gate: 3672736 stripped bytes, 3358720 text bytes; load 168.024 ms; ticks_600 25.029 ms
```

Final duplicate labels: **19 melee-ft, 7 melee-sim, 100 across crates**.
The previously reviewed ceilings are 20 / 7 / 100. All **311** audited common
labels have exactly one definition. Character duplicate counts are unchanged.
The complete current reports and machine-readable blocks are in `docs/PERF.md`.

## Changed files in this continuation

The earlier HUD and initial 846-tick prefix changes are already in the rebased
lane baseline. This continuation changes the following files:

- `TRACKER.md`
- `crates/ft-falco/src/`: `init.rs`
- `crates/ft-fox-family/src/`: `lib.rs`, `special_n.rs`, `special_s.rs`
- `crates/ft-fox/src/`: `init.rs`
- `crates/ft-mars/src/`: `init.rs`, `lib.rs`, `special_lw.rs`, `special_n.rs`, `special_s.rs`
- `crates/hsd-particle/src/`: `system.rs`
- `crates/it-foxlaser/tests/`: `flight.rs`
- `crates/melee-ef/src/`: `lib.rs`, `request.rs`, `tables.rs`
- `crates/melee-ft/src/fighter/`: `assets.rs`, `attack.rs`, `character.rs`, `commands.rs`, `damage.rs`, `dash.rs`, `down.rs`, `effects.rs`, `grab.rs`, `grab_throw.rs`, `jump.rs`, `life.rs`, `mod.rs`, `procs.rs`, `run.rs`, `shield.rs`, `smash.rs`, `spawn.rs`, `teeter.rs`
- `crates/melee-ft/src/fighter/attack/`: `combo.rs`, `stale.rs`
- `crates/melee-ft/src/fighter/state/`: `common_table.rs`, `special.rs`
- `crates/melee-ft/src/fighter/state/callbacks/`: `physics.rs`
- `crates/melee-ft/src/input/`: `common.rs`
- `crates/melee-ft/tests/input_support/`: `mod.rs`
- `crates/melee-gr/src/last/`: `animation.rs`, `background.rs`, `mod.rs`, `prepared.rs`
- `crates/melee-it/src/`: `engine.rs`, `spawn.rs`
- `crates/melee-lb/src/`: `lib.rs`, `orientation.rs`
- `crates/melee-sim/src/`: `frame.rs`, `scene_fighter.rs`, `scene_stage.rs`
- `crates/melee-sim/src/frame/`: `fd_background.rs`
- `crates/melee-sim/src/initial_state/`: `cold.rs`, `fighter.rs`, `stage.rs`
- `crates/melee-sim/src/scene_stage/`: `last.rs`
- `crates/melee-sim/tests/`: `alloc_gate.rs`, `m5_gate.rs`
- `crates/melee-types/src/`: `combat.rs`, `fixed.rs`
- `docs/`: `PERF.md`
- `docs/PORT_NOTES/`: `S11_HUMAN_MATCH.md`
