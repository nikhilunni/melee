# S7/S8: throws, pummel, mash escape and quick ledge options

2026-09-10, `lane/chars`, resumed after the S3/S4/S5/S9/S10 rebase. All ten scene CLI gates are exact, including forward throw after correcting the animation-source skeleton. M4, M5 and allocation gates pass in both profiles. Full workspace acceptance remains blocked by the empty protected decomp directory, and merge-check by `main` advancing beyond this lane. No expected values, ignores, scenarios, traces, ROMs or decomp files were changed, and no Git write commands were used.

## Scene evidence

Ticks below are zero-based trace ordinals. Startup movement is omitted. The two high-percent recordings contain 420 records; the other eight contain 300.

| Scene | Retail states after grab/ledge wait | First-divergence history | Current CLI outcome |
|---|---|---|---|
| `fthrow_fd_marth` | Marth 219@141 → 14@164; Fox 239@141 → 86@151 → 29@181 → 0@221 | Original input stop at 141; post-rebase airborne-hit stop at 151; after entry port, `p1.cur_pos.y` differed by one bit at 151; resolved by using the animation-declared source skeleton | 300 ticks exact |
| `uthrow_fd_marth` | Marth 221@141 → 14@174; Fox 241@141 → 90@150 → 183@184 → 184@210 | Original 141 animation frame, then graphics 0x3FA; post-rebase airborne-hit stop at 150; resolved by thrown airborne entry | 300 ticks exact |
| `dthrow_fd_marth` | Marth 222@141 → 14@173; Fox 242@141 → 90@151 → 183@166 → 184@192 | Original 141 animation frame; post-rebase airborne-hit stop at 151; resolved by explicit motion override | 300 ticks exact |
| `pummel_fd_marth` | Marth 217@135 → 216@162 → 218@206 → 14@236; Fox 228@141 → 227@164 → 229@206 → 14@236 | Original input/captured-hit stops; post-rebase 141 animation frame 1 instead of 0; then 142 Y drift from alignment during hitlag; then hitlag-exit interaction assertion | 300 ticks exact |
| `grabmash_fd_marth` | Marth 218@138 → 14@168; Fox 229@138 → 14@168 | Original 132 animation frame 5 instead of 4; exact before and after rebase | 300 ticks exact |
| `ledgeattack_fd_fox` | 257@231 → 14@285 | Original input stop at 231; exact before and after rebase | 300 ticks exact |
| `ledgejump_fd_fox` | 262@231 → 263@245 → 42@282 | Already exact before implementation and after rebase | 300 ticks exact |
| `ledgeroll_fd_fox` | 259@231 → 14@280 | Already exact before implementation and after rebase | 300 ticks exact |
| `hi200_uthrow_fd_marth` | Marth 221@140 → 14@173; Fox 241@140 → 90@149 → 38@225 → 191@281 → 192@307 | Post-rebase airborne-hit stop at 149; resolved by thrown airborne entry and merged S5/S9 behavior | 420 ticks exact |
| `hi200_uthrow2_fd_marth` | Marth 221@146 → 14@179; Fox 241@146 → 90@155 → 38@231 → 191@287 → 192@313 | Post-rebase airborne-hit stop at 155; resolved by thrown airborne entry and merged S5/S9 behavior | 420 ticks exact |

The current CLI compares item fields too and prints **62 keys** for these recordings. M5 separately checks all 49 fighter/RNG keys and ordered particle RNG sites. The comparator was not changed to force the original task's 49-key CLI label.

## Resumed implementation

`damage.rs` accepts ThrownF/B/Hi/Lw releases as airborne. The static airborne reaction array selects DamageAir3 for forward throw. An explicit `Option<CommonMotionState>` carries down throw's motion override into the existing damage entry. `ftCo_800DE7C0`'s misleadingly named `calcKnockbackAngle` returns a **motion ID**, not an animation-selection angle: 90 forces damage level 3 and overrides the selected motion after the existing DamageFlyRoll RNG draw. The launch descriptor's angle remains unchanged. This corrects the earlier version of this report. S9's queued heavy/medium sounds, voice handling, RNG arguments and draw order are preserved.

The captured-hit branch records when the selected attacker is the victim's captor. `grab_escape::capture_damage` applies percent and enters CaptureDamageLw without the ordinary launch or extra immediate animation step, preserving the linked pair and typed capture timer. The appended common row uses animation 256 and the per-concept completion/timer callback. Existing hit detection continues to own hitboxes, hit effects and hitlag. Pair alignment now skips hitlag, matching its retail physics callback; hitlag completion restores the pair's supported interaction instead of requiring ordinary attack scratch.

Two new 420-tick M5 gates cover the high-percent throws. Up throw and pummel were added to the allocation suite at ceiling zero. The obsolete S7 prefix test and helper were removed after all ten complete scene gates passed.

No new `CharacterCallbacks` hooks, generic fighters or generic phase callbacks were added. Character differences still use the existing `throw_variant` hook and descriptor data. Shield-hit functions and all other excluded lane files were left untouched. Forward throw reaches DeadDown after the pair has already been released, so it does not reach S9's linked-death stop; `life.rs` was not changed.

## Earlier S7/S8 work retained through the rebase

Static throw descriptors pair each captor state with its victim state, animation and weight mask. Motion remapping borrows the source animation and part tables with source masks prepared during asset loading, command streams use prepared shared immutable slices, and XRotN constraint slots survive activation/deactivation. These remove throw-entry heap clones and constraint-node allocations; the existing back-throw allocation gate remains.

`grab_escape.rs` owns capture scratch and archive-derived timer/mash/cut parameters. The timer includes percent, handicap and stock rank. Button edges and signed stick-direction changes apply the retail mash decrements, and the victim-owned timer releases both fighters into CatchCut/CaptureCut. Pummel input has priority over throws. Quick ledge attack uses an appended common row, ordinary subaction hitboxes and shared ledge physics/collision; the recorded jump and roll paths already passed.

Up-throw graphics 0x3FA now uses S3's merged ModelSpawn implementation. The earlier duplicated effect path was resolved during the user's rebase. Slow ledge choices, airborne cut, C-stick throws and unsupported character throw variants remain outside this recorded slice. The two pre-existing ignored raw back-throw hitbox-phase tests are unchanged.

## Forward-throw root cause and reviewer follow-up

The original tick-151 failure was `p1.cur_pos.y`: expected `1.5432478189468384` (`0x3FC58925`), actual `1.543247938156128` (`0x3FC58926`). X and both knockback components matched; the full actual trace had no other differing tick/key.

The cause was **the wrong animation-source part table**. At tick 150 the raw victim has `x594_s32 = 0x80000021`; its low six bits declare source skeleton 33 (`FTKIND_NONE`, the shared animation skeleton). The port selected Marth's part table because Marth owns the borrowed animation. Retail instead reads `x597_bits` from the animation flags: `8006FD08` loads byte +597, `8006FD18` keeps its low six bits, and `8006FD28` / `8006FD50` pass that source to the conditional-mask lookup and `ftPartsRemap`. `ftPartsRemap` at `80075028..80075060` maps source joint → semantic part → victim joint.

The raw Fighter snapshot supplied enough pose evidence without another Dolphin capture. Its cached HipN hurtbox endpoints match the old port exactly, but the limb endpoints differ before release. For example, bone 13's zero-offset endpoint at tick 150 is `(74.26688385, 8.19471550, 3.99870968)` in retail versus `(70.20452, 2.989437, 4.364431)` in the old port. This identifies a body-pose mismatch despite the matching saved XRotN and HipN translations.

`MotionFlags::source_skeleton` now decodes the declared source. `AnimationSource` reads its PlCo part map, including the final shared-skeleton table, and `ftParts_8007506C` conditional masks. Asset loading prepares the borrowed throw motions in the existing `Motion.remap` storage. An initial separate source-table map introduced five cross-crate destructor copies; reusing the existing owner removes that additional storage type. Throw entry borrows that source and its masks while retaining the victim's destination table. There are no per-tick allocations or character-kind branches.

### Collision audit and corrected snap chronology

Fresh instrumentation of the rebased code confirmed that both snaps were **inside the same** `air_collide_pass`; `begin_damage_reaction` did not snap again. The reviewer correctly identified the final post-call value, but that value did not reveal the internal movement substeps.

| Old port stage | Y / relevant ECB state |
|---|---|
| Release target from TransN2 + root offset | `-6.6853075` |
| Collision `last_pos` from captor ECB center | `5.9530954` |
| `mpColl_80043754` movement subdivision | Three steps; delta Y `-4.212801` |
| Second substep, first floor probe | `-2.4725065`; interpolated bottom `0` |
| First additive floor snap | `0x38D18000` (the exact retail pre-integration Y) |
| Repeated floor check in `mpColl_80046904` | Zero bottom allows another contact within retail's intersection epsilon |
| Second additive floor snap in that call | `0x38D1B717` (`0.0001f`) |

The C and assembly support this floor-crossing/probe path and the settle loop. The operand magnitude 2..4 comes from movement subdivision, even though the release target is −6.685. The old pose produced desired bottom zero; retail's recorded release ECB history has bottom `1.8834445476531982` and top `4.552462577819824`. Correcting the source skeleton fixes the pose and the complete forward-throw gate without altering mp collision branches, contact constants, FMA order or integration arithmetic.

Constraint invalidation, root-map writes and eager matrix evaluation did not resolve the mismatch. Those experiments and every diagnostic print were removed. The earlier headless probe attempt failed before capture in macOS LaunchServices; it is no longer a prerequisite. No numeric correction, tolerance or expected-value change was introduced.

## Source and arithmetic audit

The lane's decomp directory is empty. Source and retail assembly were read from `/Users/nikhilunni/Projects/melee/third_party/melee-decomp`, using the main checkout's `harness/asm.py` read-only. Neither decomp path was modified.

The resumed damage changes select existing arithmetic paths and preserve S9's RNG/sound ordering. CaptureDamage uses `fn_800DB8A4`'s f64 elapsed-counter increment, f32 timer decrement and existing audited mash helper. The earlier capture-timer setup retains fmadds at 800DA9CC/800DA9D4 with its separately rounded rank term; throw weight scaling retains its audited separate multiply/divide. Existing thrown-pose and ledge FMA sites are unchanged.

## Files changed in this resumed session

- `crates/melee-ft/src/fighter/damage.rs`: thrown airborne entry, forced motion and captured-hit routing/hitlag completion.
- `crates/melee-ft/src/fighter/grab_throw.rs`: down-throw motion override and animation-declared source remap.
- `crates/melee-ft/src/anim/playback.rs`, `desc/bones.rs`, `fighter/assets.rs`: source-skeleton decoding and prepared part maps/masks.
- `crates/melee-ft/src/fighter/grab_escape.rs`, `grab.rs`: captured damage entry, timer/completion and low-capture collision support.
- `crates/melee-ft/src/fighter/state/common_table.rs`: appended CaptureDamageLw row.
- `crates/melee-sim/src/frame/grab_pairs.rs`: skip captured alignment during hitlag.
- `crates/melee-sim/tests/m5_gate.rs`, `alloc_gate.rs`: high-percent full gates, removal of obsolete prefixes and two zero-allocation scenes.
- `docs/PERF.md`: performance gate generated evidence; no budget changes.
- `TRACKER.md`, this report: current evidence and remaining work.

## Final validation

Final commands and exit codes are recorded in `/tmp/s7-final-validation.json`, with separate command logs at `/tmp/s7-final-*.log`. Both workspace profiles completed all 184 test targets: **1,100 passed, 1 failed, 3 ignored**. The sole failure is the unmodified `dynamics_c_excerpts_match_decomp` test: the lane's empty decomp directory prevents it from finding `src/melee/lb/lb_00F9.c`. The native-C numerical oracle passes in both profiles. Running the identical excerpt comparisons read-only against the main checkout also passes (`/tmp/s7-review-main-decomp-excerpts.log`); that does not change the workspace command's failure status.

| Suite | Debug exact result line | Release exact result line |
|---|---|---|
| M4 | `test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 106.82s` | `test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.06s` |
| M5 | `test result: ok. 79 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 49.77s` | `test result: ok. 79 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.15s` |
| Allocation | `test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 18.14s` | `test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.80s` |

The separately requested `cargo test -p melee-sim --test m4_gate` also passes: `test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 97.63s`. Up throw and pummel each measure 299 simulation ticks with **0 allocations, peak 0, allocating ticks 0** in both profiles. No allocation ceiling increased.

Both `cargo gate --no-fail-fast` and `cargo gate --release --no-fail-fast` exit 101 with the same final diagnostic:

```text
error: 1 target failed:
    `-p melee-lb --test dynamics_ref_oracle`
```

| Other required command | Final result |
|---|---|
| `cargo build -q` | PASS, exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS, exit 0 |
| `cargo fmt --all --check` | PASS, exit 0 |
| `tools/check-release-math.sh` | PASS, exit 0; debug/release math oracles and all six optimization levels |
| `tools/perf-gate.sh` | PASS, exit 0; final standalone run `20260910T153747Z-33219` |
| `tools/merge-check.sh lane/chars` | Exit 1 after both data checks pass: `[FAIL] rebase: main is not an ancestor of lane/chars (or ref lookup failed)` |

At final validation `main` is `1994cff`, while this lane remains at `22ba99f` with the edits uncommitted. The requested no-Git-write boundary prevents rebasing here. No source test was redirected and no protected directory was populated to hide the environment failure. The owner must restore the lane's decomp checkout and rebase before rerunning the complete merge chain.

### Exact final CLI output

Every scene was run as `cargo run -q --release -p melee-sim -- gate harness/scenarios/<scene>.toml` after the source-skeleton correction.

| Scene | Exact final line |
|---|---|
| `fthrow_fd_marth` | `300 ticks, 62 keys, 0 divergences` |
| `uthrow_fd_marth` | `300 ticks, 62 keys, 0 divergences` |
| `dthrow_fd_marth` | `300 ticks, 62 keys, 0 divergences` |
| `pummel_fd_marth` | `300 ticks, 62 keys, 0 divergences` |
| `grabmash_fd_marth` | `300 ticks, 62 keys, 0 divergences` |
| `ledgeattack_fd_fox` | `300 ticks, 62 keys, 0 divergences` |
| `ledgejump_fd_fox` | `300 ticks, 62 keys, 0 divergences` |
| `ledgeroll_fd_fox` | `300 ticks, 62 keys, 0 divergences` |
| `hi200_uthrow_fd_marth` | `420 ticks, 62 keys, 0 divergences` |
| `hi200_uthrow2_fd_marth` | `420 ticks, 62 keys, 0 divergences` |

### Performance audit

The first source-skeleton implementation stored an additional map in `FighterAssets`. Its five new cross-crate destructor copies exceeded the zero-growth budget. Reusing `Motion.remap` removed all five. The final census is:

| Crate | Labels | Definitions | Duplicate labels |
|---|---:|---:|---:|
| `melee-ft` | 874 | 908 | 19 |
| `melee-sim` | 102 | 129 | 7 |
| `ft-captain` | 43 | 66 | 1 |
| `ft-falco` | 44 | 67 | 1 |
| `ft-fox` | 44 | 67 | 1 |
| `ft-fox-family` | 0 | 0 | 0 |
| `ft-mario` | 0 | 0 | 0 |
| `ft-mars` | 44 | 67 | 1 |
| `ft-peach` | 44 | 67 | 1 |
| `ft-purin` | 43 | 66 | 1 |
| `ft-yoshi` | 45 | 68 | 1 |

Across crates: **100 duplicate labels**, equal to the reviewed ceiling. Stripped size is **3,621,408 bytes**, text **3,309,568 bytes**; size and every copy budget pass.

Timing history is retained in the generated `docs/PERF.md` entries, including failures. The separate-map run measured load 249.597 ms / ticks 23.831 ms; the first existing-owner run measured 199.802 / 24.182 ms; restoring the original asset-load order measured 205.798 / 37.041 ms. No tolerance, benchmark or budget was changed. Alternating six runs each of the saved passing binary and the final binary on the same 600-tick scene measured mean CPU time 295.470 versus 296.002 ms (all twelve runs exact). That comparison showed little CPU-cost change but did not substitute for the required wall-time gate.

After all acceptance test processes finished, the standalone gate passed with the same final source: load **164.696 ms**, 600 ticks **23.358 ms**, below the fixed 182.600 / 25.947 ms ceilings. Evidence is in `target/perf/20260910T153747Z-33219`, `docs/PERF.md`, and `/tmp/s7-final-standalone-perf.log`. Exact final line:

```text
[PASS] perf-gate: 3621408 stripped bytes, 3309568 text bytes; load 164.696 ms; ticks_600 23.358 ms
```
