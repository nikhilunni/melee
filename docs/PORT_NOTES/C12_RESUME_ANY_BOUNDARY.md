# C12: saved scheduler cursor and partial-callback continuations

2026-09-09, chars lane. No commits, Git writes, recordings, or changes to the
protected harness/decomp paths. The decomp symlink/type change was present before
this task.

## Result and scope

All 26 available idle/start/cold scenarios in the requested character/stage scope
pass 600 ticks × 49 keys with zero divergences. The stock-2 savestate also imports:
its available `ko_fd_marth` gate passes 480 ticks × 49 keys. There is no
`idle2_fd_marth.toml` or standalone `idle2_fd_marth.tick.raw.jsonl` /
`tick.expected.jsonl` in either the lane or main checkout, so a separate 600-tick
idle2 oracle comparison cannot run without Claude recording it.

The cursor is independent of s_link and fighter index. Callback entry and return
boundaries work without callback-specific cases. **This is not a general PowerPC
continuation interpreter:** interior PCs still require an audited continuation.
All interior boundaries in the supplied recreated corpus, including stock 2, are
handled; arbitrary other interior PCs and pending scheduler mutations remain
explicit errors. Existing Wait/Entry fighter-state and stage-content import
restrictions remain. Thus literal acceptance of every possible instruction/state
is not claimed.

## Resume model and memory contract

`SchedulerResume` contains the saved s_link, current callback/owner identity,
completed prefix of that link's actual doubly linked proc list, and a typed
continuation for the current invocation. `ProcKey` includes p_link, owner identity
(player id or Ground map id), and callback address. Two procs on one Ground object
remain distinct. Same-link fighter 0 and fighter 1 remain distinct.

On tick zero, dispatch skips earlier links and the saved completed prefix, finishes
the interrupted callback, then visits later registrations using the existing HSD
world in retail order. Subsequent ticks use ordinary dispatch. The resume path adds
no allocation to ordinary ticks. No later trace rows, oracle seeds, or RNG ledgers
enter runtime state.

Retail references below are relative to `third_party/melee-decomp/src/`.

| Saved location | Meaning | Retail reference |
|---|---|---|
| `804D7834` | current s_link, **not a list head** | `sysdolphin/baselib/gobj.c:20,101-103`; `gobjproc.c:90-94` |
| `804D7838` | currently executing HSD_GObjProc | `gobj.c:19,113-115,135`; `gobjproc.c:90-94,169-170` |
| `804D7830` | next proc selected by the scheduler | `gobj.c:21,105,115,138`; `gobjproc.c:90-94,101-103` |
| `804D781C` | current GObj | `gobj.c:26,108-114,134` |
| `804D783C` | modulo-three visited epoch | `gobj.c:18,96-107`; `gobjproc.c:158-160` initializes a fresh proc to epoch 3 |
| `804D7840` | pointer to array of proc-list heads, indexed by s_link | `gobj.c:17,103`; `gobjproc.c:71-73,121` |
| `804D7844` | per-p_link insertion tails, not the traversal-head array | `gobj.c:16`; `gobjproc.c:28-64,104-115` |
| `804CE3E4` | deferred unlink/destroy/relink flags | `gobj.c:116-132`; `gobjproc.c:90-94,169-174` |
| proc `+00/+04/+08` | owner-child / scheduler-next / scheduler-previous links | `gobjproc.h:9-11`; `gobjproc.c:71-89,128-140` |
| proc `+0C/+0D` | s_link and packed disable/epoch flags | `gobjproc.h:12-16`; `gobjproc.c:158-160`; `gobj.c:106-110` |
| proc `+10/+14` | GObj owner and callback | `gobjproc.h:17-18`; `gobjproc.c:161-162` |
| GObj `+02/+04/+2C` | p_link / priority / user data | `gobj.h:30-45`; insertion order in `gobjproc.c:19-64` |

Checks retain the original seed/particle metadata agreement, and add current-GObj,
next-proc, visited-epoch, disable-flag, bounded list traversal, next/previous
consistency, and current-proc membership checks. The current cursor must also map to exactly one
modeled registration, so unknown callbacks cannot be silently omitted. CPU/stack provenance constrains
interior continuations. Stack walks are bounded, aligned, and strictly ascending.

At `80390DF0..80390DFC` (before invocation), a callback's entry PC, and
`80390E00..80390E10` (after return), r27/r28/r24 must agree with proc/link/owner.
These are the assembly sites implementing `gobj.c:112-115`. An interior save uses
its stack to identify the unfinished subroutine; current-proc metadata alone is
insufficient because `on_invoke` has not returned yet.

The old constructor comparison against completed tick zero still applies to
late boundaries. For earlier links that still owe animation/collision, it belongs
to the ordinary frame-zero gate after continuation, not before unfinished work.
Saved fighter kind/state/slot are checked independently. No expected values changed.

## Boundary analysis

Fighter registrations are `melee/ft/fighter.c:897-911`: status 0, animation 1,
CPU gate 2, input 3, physics 4, collision 6, pose 7, accessories 8, hitbox positions
9, grab 12, hit detection 13, ProcessHit 14, dynamics 16, camera 18, mirror 22.
**8006D1EC is ProcessHit, not the camera proc.** Camera is 8006D9EC at link 18
(`fighter.c:2806,3061`).

| Save(s) | Saved link / current owner / PC | Already completed | Continuation and remaining work |
|---|---|---|---|
| `idle_fd_fox` | 24 / none / `80019230` | preceding whole tick | One full tick, preserving the existing idle-between-ticks behavior |
| `idle_bf_fox` | 1 / fighter 1 / `8036B66C` | status for both; Ground animation; fighter 0 animation; fighter 1 counters and blend progress increment | Finish secondary-tree evaluation and blend once, then commands/Wait tail; both fighters' input/physics/collision and all later stage/particle phases |
| `idle_fd_falco` | 1 / fighter 1 / `8036B710` | same prefix; fighter 1 already selected/restarted Wait and entered its initial blend evaluation | Finish selected animation at frame 0 and restart commands, without repeating Wait selection or RNG; later phases normally |
| `idle_fd_peach` | 1 / fighter 0 / `8036B570` | status for both and Ground animation; fighter 0 entered a Wait restart | Finish fighter 0 restart, run fighter 1's animation, then both fighters' later phases |
| `idle_fd_yoshi` | 1 / fighter 1 / `80364B70` | fighter 0 animation; fighter 1 Wait selection and animation requests | Finish the new blend/rate setup and initial evaluation; remaining fighter/stage/particle procs |
| `idle_dl_fox`, `idle_fd_marth` | 6 / fighter 1 / `803263BC`, `8037A478` | all animation/input/physics and stage link 4; fighter 0 collision; current fighter collision position/lock prologue | Finish pure ECB fitting and remaining collision, preserving saved history; later fighter/particle procs |
| `idle_ys_fox` | 6 / fighter 0 / `803261B4` | all animation/input/physics and stage link 4; fighter 0 collision prologue | Finish fighter 0 collision, run fighter 1 collision, then later phases |
| `idle_fd_falcon` | 6 / fighter 1 / `8004DD08` | same earlier phases; fighter 0 collision; fighter 1 ECB interpolation and environment-history rollover | Resume the final contact-free geometry step, retain prior ECB/environment history, finish collision tail and later procs |
| `idle2_fd_marth` via `ko_fd_marth` | 6 / fighter 0 / `80050F68` | all earlier phases; fighter 0 ECB interpolation/history rollover | Same final-step continuation, at a right-wall query; then fighter 1 collision and later procs |
| `idle_fd_puff` | 4 / Ground map 3 / `8000C17C` | both animation/input procs; Ground maps 0–2 and map 3's wrapper/controller phase advance | Finish map 3 collision tail; run maps 4–8 and both fighters' physics, then collision and particles |
| `start_fd_puff` | 24 / no scheduler proc / `8037F210` | stage construction and fighter 0 creation | Finish fighter 1 creation from saved StaticPlayer state, then music selection; no gameplay tick is owed before initial observation |
| other available start scenes | 24 / no scheduler proc | scene creation, or existing supported costume-allocation continuation | Existing creation/music boundary; subsequent ticks are full ticks |

Animation evidence: `fighter.c:1683-1698`, `ftanim.c:314-376,380-385,388-428`.
`8006EB18` is after the secondary-tree evaluation but before its blend into the main
pose. The saved main pose is therefore usable; archive tracks can be requested at
the unfinished target frame without repeating the blend. The captured increment
at `8006EA70` is accounted for once. The Wait-restart callers are `80069E88` and
`8006AB78`; the rate-configuration case returns at `8006ED84` through `80069E78`.
Root motion and active independent-part animations are explicitly excluded from
this specialized continuation. Float operations reuse the audited playback code.

Collision evidence: `fighter.c:2476-2516`, `ft_081B.c:1081-1100`,
`mp/mpcoll.c:87-113,2685-2701,3855-4031`. The early saves are inside ECB bone
queries, before geometry steps. The sweep saves are inside pure line probes with
no contact committed: `8004ADC0/8004DD08` and `8004ADF8/80050F68`. The
`mpColl_8004ACE4` frame saves step count/index at `+38/+3C`; checks require 1/0,
zero environment flags, no contact/iteration flag, stationary position, and an
unlocked Wait. Already interpolated `ecb`, `prev_ecb`, and `prev_env_flags` are
retained. A new full-CollData regression compares resumed and uninterrupted
collision with a changing ECB, so replaying interpolation cannot silently pass.

Ground ownership: `gr/grlast.c:165` registers `grLast_8021AAB0` for map 3.
Its controller call precedes `Ground_801C2FE0` and `lb_800115F4`
(`grlast.c:362-375`). The saved stack returns at `8021AB14` / `801C3068`,
inside collision-transform work, so running the controller again would advance
its timer twice. Resume performs collision work only and requires an empty quake
list. The remaining Ground maps and fighters are selected by actual list position.

Creation evidence: `pl/player.c:203-240` calls `Fighter_Create`;
`ft/fighter.c:846-932` loads costume joints before gameplay reset.
`8037F210` is the return from OSAllocFromHeap in HSD_MemAlloc
(`baselib/memory.c:14-25`), via HSD_SListAlloc (`list.c:28-35`) while loading
PObj skinning envelopes (`pobj.c:202-225,294`). The stack continues through
lbRefract_PObjLoad, HSD_DObjLoadDesc and JObjLoad to `80068F8C` in Fighter_Create.
The allocation result must be a MEM1 pointer. Existing creation-stack provenance,
human-player and RNG consistency checks remain; saved CPU initialization is not
replayed for the already created fighter.

## Removed and retained restrictions

- Removed the fixed fighter-0 ownership assumption and the two-pair scheduler
  dispatch rule. Current GObj and the saved list determine the first unfinished
  fighter or map.
- Removed `physics resume requires Wait` from the `8006BF28` tail. No further
  integration occurs there; the inactive knockback magnitude is checked directly.
  `ftColl_8007AF28` (`ftcoll.c:3092-3099`) invalidates hurt-capsule caches, which
  the port rebuilds in its later collision/hitbox phases. General fighter state
  import still supports Wait/Entry; this change does not add other state payloads.
- Retained specialized Wait/idle restrictions where restarting arbitrary state
  transitions would repeat effects, RNG, lock decrements or collision histories.
- Retained particle-emission CPU/stack/population checks. `ParticleEmission` is
  distinguished from a fresh particle callback entry or completed callback return.
- Retained rejection of unknown interior instructions and pending GObj mutations;
  no missing state or unsupported effect is approximated.

## Gate table

The lane has no `harness/roms` or `harness/traces` directories. Scenario commands
use this lane's binary and absolute scenario paths under
`/Users/nikhilunni/Projects/melee/harness/scenarios`. Fixture-backed Cargo tests
use an exact source copy under `/tmp/c12-verify`, with its harness pointing at the
main checkout's existing data. No game data was copied or rewritten.

| Scenario | Result |
|---|---|
| `idle_bf_fox` | 600 ticks, 49 keys, 0 divergences |
| `idle_dl_fox` | 600 ticks, 49 keys, 0 divergences |
| `idle_fd_falco` | 600 ticks, 49 keys, 0 divergences |
| `idle_fd_falcon` | 600 ticks, 49 keys, 0 divergences |
| `idle_fd_fox` | 600 ticks, 49 keys, 0 divergences |
| `idle_fd_marth` | 600 ticks, 49 keys, 0 divergences |
| `idle_fd_peach` | 600 ticks, 49 keys, 0 divergences |
| `idle_fd_puff` | 600 ticks, 49 keys, 0 divergences |
| `idle_fd_yoshi` | 600 ticks, 49 keys, 0 divergences |
| `idle_ys_fox` | 600 ticks, 49 keys, 0 divergences |
| `start_bf_fox` | 600 ticks, 49 keys, 0 divergences |
| `start_bf_fox_cold` | 600 ticks, 49 keys, 0 divergences |
| `start_dl_fox` | 600 ticks, 49 keys, 0 divergences |
| `start_dl_fox_cold` | 600 ticks, 49 keys, 0 divergences |
| `start_fd_falco` | 600 ticks, 49 keys, 0 divergences |
| `start_fd_falco_cold` | 600 ticks, 49 keys, 0 divergences |
| `start_fd_falcon` | 600 ticks, 49 keys, 0 divergences |
| `start_fd_fox` | 600 ticks, 49 keys, 0 divergences |
| `start_fd_fox_cold` | 600 ticks, 49 keys, 0 divergences |
| `start_fd_marth` | 600 ticks, 49 keys, 0 divergences |
| `start_fd_marth_cold` | 600 ticks, 49 keys, 0 divergences |
| `start_fd_peach` | 600 ticks, 49 keys, 0 divergences |
| `start_fd_puff` | 600 ticks, 49 keys, 0 divergences |
| `start_fd_yoshi` | 600 ticks, 49 keys, 0 divergences |
| `start_ys_fox` | 600 ticks, 49 keys, 0 divergences |
| `start_ys_fox_cold` | 600 ticks, 49 keys, 0 divergences |
| `idle2_fd_marth` | Standalone scenario/600-tick trace absent; `ko_fd_marth` using this save: 480 ticks, 49 keys, 0 divergences |

## Tests and unrelated failures

- Fixture-backed debug `m4_gate`: **261 passed, 0 failed**. Final-source release
  rerun: **261 passed, 0 failed**, with `--nocapture`; no missing-data skips.
- Final-source debug `m5_gate`: **6 passed, 2 failed**. Final-source release
  `m5_gate`: **5 passed, 3 failed**. No missing-data skips.
- Six new focused regressions cover either fighter as current owner, distinct
  callbacks on the same Ground object, callback entry/return boundaries, corrupt
  proc links/current membership, a cursor resolving to a modeled callback, and resumed collision preserving full CollData
  history. All pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: passes.
- `cargo fmt --all`: completed.
- `cargo gate` in the lane: passes (947 passed, 0 failed, 3 ignored across 149
  result blocks), **with the existing missing-data early returns** because the
  lane has no roms/traces. This is not presented as a fixture-backed green gate.
- Additional fixture-backed `cargo gate --release --no-fail-fast`: **920 passed,
  27 failed, 3 ignored**, across 149 result blocks; **21 failing targets**.
  The detailed assertion list follows below.

Unrelated failures were not fixed and no expectations were changed:

| Test / location | Evidence and reason |
|---|---|
| `jab_fd_fox_300_ticks_and_ordered_particle_draws`, `crates/melee-sim/tests/m5_gate.rs:59` | At frame 108, fighter 1 HitDetection requests `unsupported ef particle 0/6`. The existing allowlist at `crates/melee-sim/src/effects.rs:57` / guard at `:521-523` does not implement it. This scene uses the unchanged between-ticks `idle_fd_fox` import. |
| `jab_fd_marth_300_ticks_and_ordered_particle_draws`, `crates/melee-sim/tests/m5_gate.rs:7` | The 300-tick, 49-key gate and every tick's ordered particle RNG sites pass. The new ledger totals **9783**, while the old self-authored aggregate assertion still expects **9373**. |
| Release only: `utilt_fd_marth_300_ticks_and_ordered_particle_draws`, `crates/melee-sim/tests/m5_gate.rs:59` | First difference at frame **147**, `p1.kb_vel.y`: expected `80000000` (-0), actual `00000000` (+0). Debug passes all 300 ticks. This is reproducible without a savestate, scheduler, or simulator: the unchanged `gekko-math/src/fma.rs:38-39` expression for `fnmsubs` changes signed zero under optimization on this machine. The runtime caller is `melee-ft/src/fighter/damage.rs:390`. |

Independent optimized signed-zero reproduction (scratch file only):

```rust
fn main() {
    let (a, c, b) = std::hint::black_box((0.051_f32, 0.0_f32, 0.0_f32));
    let result = -a.mul_add(c, -b);
    println!("{:08X}", result.to_bits());
}
```

`rustc ... -C opt-level=0` prints `80000000`;
`rustc ... -C opt-level=3` prints `00000000`. This reproduces the relevant
arithmetic/profile difference without any C12 code. It warrants a separate math /
code-generation investigation; no compensating change is made here.

Verification logs are local scratch files: `/tmp/c12-scenario-gates-final.txt`,
`/tmp/c12-m45.log` (debug), `/tmp/c12-m45-final.log` (release),
`/tmp/c12-m5-debug-final.log`, `/tmp/c12-clippy-final.log`,
`/tmp/c12-final-lane-gate.log`, and `/tmp/c12-workspace-data-final.log`.

## Changed files

- `crates/melee-sim/src/initial_state/scheduler_resume.rs`: cursor, validation,
  continuation classification, and four scheduler regressions.
- `crates/melee-sim/src/initial_state/mod.rs`: cursor import, saved fighter checks,
  and early-boundary comparison placement.
- `crates/melee-sim/src/initial_state/saved_pose.rs`: bounded saved-stack walk.
- `crates/melee-sim/src/initial_state/setup_resume.rs`: envelope allocation PC and
  allocation-result check.
- `crates/melee-sim/src/initial_state/cold.rs` and `cold_tests.rs`: typed cursor;
  the existing snapshot key and expectations are unchanged.
- `crates/melee-sim/src/frame.rs`: registration identities and one-tick continuation
  dispatch.
- `crates/melee-ft/src/fighter/procs.rs`: interrupted Wait animation continuation.
- `crates/melee-ft/src/collision/ground.rs`: interrupted stationary Wait collision.
- `crates/melee-mp/src/mpcoll.rs`: remaining collision step / ECB continuation.
- `crates/melee-mp/src/tests.rs`: full collision-history regression.
- `TRACKER.md` and this report: status, evidence, and remaining limits.

## Additional fixture-backed workspace findings

The release workspace run reports 21 failing targets. Beyond the M5 failures
above, these are the observed assertions. Independent leaf/subsystem tests do
not use the C12 importer. This task did not change their fixtures or expectations;
particle field mismatches have not been individually root-caused.

| Test | Location | Observed failure |
|---|---|---|
| `msl_matches_native_c_bit_for_bit` | `crates/gekko-math/tests/ref_oracle.rs:363:9` | assertion `left == right` failed: fmodf bit mismatches vs native C:; fmodf(6e0, 3e0): rust 00000000 (0e0) vs c 80000000 (-0e0); fmodf(0e0, 0e0): rust 00000000 (0e0) vs c 80000000 (-0e0) |
| `mtx_and_quat_match_native_c_bit_for_bit` | `crates/hsd-anim/tests/mtx_oracle.rs:1135:5` | Native C signed-zero / vector math differences; see full mismatch list in log. |
| `fusion_changes_results_within_the_sweep` | `crates/hsd-anim/tests/mtx_oracle.rs:1135:5` | Native C signed-zero / vector math differences; see full mismatch list in log. |
| `battlefield_idle_600_particles_and_ordered_rng` | `crates/hsd-particle/tests/support/dust_replay.rs:255:9` | assertion `left == right` failed: tick 161 field coverage;   left: 5564;  right: 5456 |
| `dream_land_idle_600_particles_and_ordered_rng` | `crates/hsd-particle/tests/support/dust_replay.rs:255:9` | assertion `left == right` failed: tick 533 field coverage;   left: 47;  right: 20 |
| `live_fd_600_ticks_match_rng_counts_sites_and_seeds` | `crates/hsd-particle/tests/live_fd.rs:50:5` | assertion `left == right` failed;   left: 3182633190;  right: 1286746018 |
| `live_fd_every_dumped_frame_matches_every_field` | `crates/hsd-particle/tests/live_fd.rs:50:5` | assertion `left == right` failed;   left: 3182633190;  right: 1286746018 |
| `live_fd_jab_marth_300_ticks_match_every_field_and_rng_draw` | `crates/hsd-particle/tests/support/dust_replay.rs:283:5` | Particle generator / position / velocity field mismatches (full field counts in log). |
| `live_fd_ledge_420_ticks_match_every_field_and_rng_draw` | `crates/hsd-particle/tests/support/dust_replay.rs:228:9` | assertion `left == right` failed: tick 245 draw count;   left: 43;  right: 58 |
| `live_fd_start_600_ticks_match_every_field_and_rng_draw` | `crates/hsd-particle/tests/live_fd_start.rs:25:14` | unclassified draw 0x801c26ac |
| `jab_fd_fox_particles_300_ticks` | `crates/hsd-particle/tests/support/dust_replay.rs:226:13` | assertion `left == right` failed: tick 108 draw 37;   left: 2151270612;  right: 2151270512 |
| `ko_fd_marth_particles_480_ticks` | `crates/hsd-particle/tests/support/dust_replay.rs:283:5` | Particle generator / position / velocity field mismatches (full field counts in log). |
| `utilt_fd_marth_particles_300_ticks` | `crates/hsd-particle/tests/support/dust_replay.rs:283:5` | Particle generator / position / velocity field mismatches (full field counts in log). |
| `story_idle_600_particles_and_ordered_rng` | `crates/hsd-particle/tests/support/dust_replay.rs:228:9` | assertion `left == right` failed: tick 11 draw count;   left: 0;  right: 3 |
| `story_start_600_particles_and_ordered_rng` | `crates/hsd-particle/tests/support/dust_replay.rs:226:13` | assertion `left == right` failed: tick 27 draw 40;   left: 2151277028;  right: 2151280384 |
| `idle_fox_600` | `crates/melee-ft/tests/fighter_support/saved_pose.rs:73:9` | assertion `left == right` failed;   left: [64, 160, 0, 0, 0, 0, 0, 0, 63, 128, 0, 0, 63, 128, 0, 0, 64, 192, 0, 0, 64, 192, 0, 0];  right: [64, 192, 0, 0, 0, 0, 0, 0, 63, 128, 0, 0, 63, 128, 0, 0, 64, 192, 0, 0, 64, 192, 0, 0] |
| `fox_wait_playback_600` | `crates/melee-ft/tests/real_fox_wait_playback.rs:121:5` | assertion `left == right` failed;   left: [1086324736, 1065353216];  right: [1084227584, 0] |
| `start_fox_600` | `crates/melee-ft/tests/fighter_support/replay.rs:302:9` | assertion `left == right` failed;   left: 15;  right: 16 |
| `idle_fox_bones_8` | `crates/melee-ft/tests/fighter_support/saved_pose.rs:73:9` | assertion `left == right` failed;   left: [64, 160, 0, 0, 0, 0, 0, 0, 63, 128, 0, 0, 63, 128, 0, 0, 64, 192, 0, 0, 64, 192, 0, 0];  right: [64, 192, 0, 0, 0, 0, 0, 0, 63, 128, 0, 0, 63, 128, 0, 0, 64, 192, 0, 0, 64, 192, 0, 0] |
| `start_fox_state_callbacks_600` | `crates/melee-ft/tests/start_fox_states.rs:140:5` | assertion `left == right` failed;   left: 15;  right: 16 |
| `frame::combat::fox_jab_hitboxes_hitlag_and_hitstun_match_retail_scratch` | `crates/melee-sim/src/frame/combat.rs:160:27` | called `Result::unwrap()` on an `Err` value: frame 108 proc Registration { s_link: 13, p_link: 8, priority: 0, object: 1, callback: Fighter { player: 1, proc: HitDetection } }: unsupported ef particle 0/6 |
| `frame::falcon_bones::idle_falcon_partial_emission_particles_600` | `crates/melee-sim/src/frame/falcon_bones.rs:138:5` | assertion failed: initial.pending_emission.is_some() |
| `frame::start_tests::start_effect_matrices_and_particle_state` | `crates/melee-sim/src/frame.rs:866:9` | assertion failed: ledger["rng_draws"].as_array().unwrap().is_empty() |
| `ledger_links_tick_end_to_next_scheduler_start_even_with_repeated_vi_frames` | `crates/melee-sim/tests/slippi_oracle.rs:95:5` | assertion `left == right` failed;   left: Number(772060177);  right: 3427901605 |
