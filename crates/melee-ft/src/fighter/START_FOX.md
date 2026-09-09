# Fox match-start callback port

The requested state callbacks match both Foxes for all 600 records, with all
24 fields per fighter and all 16 fighter RNG draws. **Full-proc acceptance is
not complete.** `start_fox_600` still calls every scheduler proc and stops at
tick 1, P0, s_link 16, Entry (322), on the existing active-dynamics guard:
`lb_00F9.c:472-935`, `lb_8001044C` (0x8001044C).

The initial `dynamic_bone_sets[0].bone_id` (+2F0) is `0x00000000`, enabling the
Fox tail chain rooted at bone 17. Idle's `0x00000100` disables the solver.
Neither the imported value nor the guard was changed. The complete test has
compared only tick 0 (1/600 records per player). There is no differing recorded
float to report at this stop: it is an unimplemented proc, before tick 1's
frame-end comparison. The tail also contains a hurtbox (bone 18); a full pose
or interaction claim would require its solver and initial heap state.

`start_fox_state_callbacks_600` is a separate, deliberately narrower test of
Status/Anim/Input/Phys/Coll in s_link order. It omits later scheduler procs and
must not be used as a substitute for `start_fox_600`. It matches P0 600/600 and
P1 600/600, all fields, with no first bit mismatch. It additionally checks the
submotion ID and entry timers against raw bytes. Later raw/expected rows feed
assertions only; the explicitly scoped external RNG seeds are the sole
per-tick oracle inputs.

## State entries and gates

C paths below are relative to `third_party/melee-decomp/src/melee/`.

| State / entry | Callback addresses: Anim / IASA / Phys / Coll | Transition gate |
|---|---|---|
| Entry 322, `ftCo_800C61B0` 800C61B0 | 800C6370 / 800C63B4 / 800C63B8 / 800C6404 | `ft/ft_0C31.c:50-57`: timer is checked before decrement; zero enters EntryStart, then decrements its replacement timer |
| EntryStart 323, `ftCo_800C6408` 800C6408 | 800C6700 / 800C673C / 800C6740 / 800C6950 | `ft_0C31.c:137-144`: decrement PlCo +6BC timer; zero enters EntryEnd |
| EntryEnd 324, `ftCo_800C6B6C` 800C6B6C | 800C6CC8 / 800C6D34 / 800C6D38 / 800C6E90 | `ft_0C31.c:267-277`: decrement PlCo +6C0 timer; zero calls `ftCommon_8007D92C`; `ftcommon.c:596-604` selects Fall if airborne, Wait otherwise |
| Fall 29, `ftCo_Fall_Enter` 800CC730 | 800CCA00 / 800CCD34 / 800CCD58 / 800CCD78 | `ftCo_Fall.c:208-211`, `ft_081B.c:685-695`: `ft_800831CC` floor collision calls `ft_80082B1C`; lines 504-512 choose Landing for sufficiently negative vertical speed, otherwise Wait |
| Landing 42, `ftCo_Landing_Enter` 800D5AEC | 800D5D3C / 800D5D78 / 800D5F18 / 800D5F38 | `ftCo_Landing.c:115-120`: animation exhaustion enters Wait. Lines 122-149 gate input interruptions using `normal_landing_lag`; neutral input plays the entire 30-frame animation |
| Wait 14 | Existing T10/T7 callbacks | Existing animation choice/restart behavior retained |

The Entry callbacks do **not** inspect the GO timer. `gm_801A4D34`
(`gm/gm_1A45.c:313-340`) calls the scene and then the scheduler; the Vs callback
(`gm/gm_16AE.c:1527-1553`) handles scene countdown/match state separately. The
fighter's entry delay is supplied by `Player_GetUnk4C`; both following local
PlCo durations are 30. From the imported boundary, the transitions are:

| Fighter | EntryStart | EntryEnd | Fall | Landing | Wait |
|---|---:|---:|---:|---:|---:|
| P0 | 6 | 35 | 65 | 75 | 105 |
| P1 | 11 | 40 | 70 | 80 | 110 |

The task description's frame-hold detail was reversed: **323 holds at 10**;
**322 and 324 record -1**. `ftmotionstates.c:3677-3707` gives EntryStart
submotion 238 and the other two SM_None. `Fighter_ChangeMotionState`,
`fighter.c:1224-1232,1349-1357`, initializes frame to start minus speed and
removes AObjs for SM_None while retaining the pose. No tick-index special case
or explicit frame-10 clamp was introduced.

## Physics, collision, effects

- `physics/airborne.rs` ports `ftCommon_Fall` (8007D494),
  `ftCommon_8007D28C` / `8007D174` and `ApplyFrictionAir` (8007CE94).
  Gravity subtracts before terminal clamping. Neutral drift uses the air
  friction >= comparison, including the sign of zero. Velocity integration
  remains the existing `Fighter_procUpdate` arithmetic.
- `collision/air.rs` routes Fall through `mpColl_80047E14` with neutral platform
  acceptance; Entry uses the captured fixed ECB and `mpColl_8004730C`. Floor
  snap and swept ECB resolution are computed by melee-mp, not a y=0 test.
- `leave_ground` is `ftCommon_8007D5D4` (8007D5D4, ftcommon.c:515-525): one
  jump used, zero ground velocity, ten-frame ECB lock. `land` is the neutral
  `8007D7FC -> 8007D6A4` path (ftcommon.c:550-595): clamp horizontal speed,
  ground velocity, zero jumps, unlock ECB. Landing preserves vertical self
  velocity on the collision tick; the next ground Phys callback overwrites it.
  `8007CB74` is ground movement projection, not the land helper.
- `proc_map_with_assets` supplies resources and RNG for immediate motion entry.
  The old grounded `proc_map` API remains available to melee-sim.
- Entry warp is a **common** `efAsync_Spawn(...,3,0x43E,root,scale)` request
  (`ft_0C31.c:129-131`), not a Fox-specific callback. `EffectSink` and
  `Fighter::drain_effects` expose it without implementing particles.
- Landing opcode 55 (`ftAction_80072E4C`, ftaction.c:1195-1242) records its
  effect request. The reachable FD effect is 0x404. `ftCo_8009F834` consumes
  three offset draws even with zero range: lr-4 = 8009FCDC, 8009FD00, 8009FD24,
  at ticks 75 and 80. Together with ten Wait-choice draws (8008A8BC), the
  focused replay verifies 16 draws. Particle/scene RNG is not certified here.

Entry accessories are represented by the scale/lift that affect fighter
physics, plus a particle request. No accessory mesh is rendered. Directional
Fall animation, C-stick aerial selection, fast fall, aerial damage, non-neutral
transition bodies, scaled-player modifiers, active tail dynamics and scene
input-thaw timing remain outside this port. The original full-proc guard is
retained; no bone/pose parity is claimed.

## Initial boundary

The new replay reads the expected, raw and RNG-ledger files. It imports both
fighters from raw **tick zero only**, then checks that Snapshot exactly equals
the initial expected row. It never calls cold spawn or CPU initialization.

Imported: player/costume, position/facing/percent, self/knockback/shield and
animation velocities, previous position/delta, ground speeds/accelerations,
ground/air and jumps, input histories/timers/freeze, seven CPU values,
shield/name-tag state, thrown-capsule fields, ground-pose flags, dynamics
sentinels, command scalars and full CollData history. Entry additionally imports
its timer (+2340), spawn height (+2344), original/current scales (+2348/+2354),
trophy height/scale/lift (+2360/+2364/+2368), and fixed ECB (+236C..2380).
Unused trophy values and command-return bytes in Entry contain old union data;
they are not read before their initialization. A null command pointer means
there is no active return stack to import.

Computed: owned structs and callback identities, archive attributes/common
durations, costume rest skeleton and bone bindings, collision map, animation
resources and playback, entry scale/lift, gravity/drift, floor snap, landing,
Wait transitions and choices. No savestate pose heap is imported by this new
test; the original idle test still imports its saved pose unchanged. That is
sufficient for the narrower 24-field callback replay, not full tail dynamics.

## Arithmetic audit and tests

`asm.py --fused` was run on EntryStart/End Phys and entry helpers, Fall blending,
Fighter_ChangeMotionState, Fall/air drift/friction, land helpers, and effect
request arithmetic. New arithmetic sites:

- Entry scale: `800C67AC fmadds`.
- Trophy height: double `1.497345 * trophy_scale`, then one f32 rounding.
  Lift is a separate f32 multiply followed by a stored-value addition.
- Air drift: `8007D2AC fmuls`, `8007D2C8 fadds`, deliberately unfused.
- Landing offset: `8009FCF8`, `8009FD1C`, `8009FD44` fmadds.
- Neutral Fall blend's `800CCCA4 fmadds` remains zero; nonzero drift blending
  is explicitly rejected until FallF/B is ported.

`airborne_ref_oracle` compiles four unmodified C excerpts with contraction off
and compares gravity, drift acceleration and integrated horizontal velocity
for 100,000 inputs. It also checks excerpt identity against the decomp.
Unit tests pin entry pre/post-decrement, completed animation hold, SM_None
frame behavior, and landing interruption count. `idle_fox_600` still matches
both fighters for 600 records and all nine Wait draws.

Final checks on this checkout:

- `cargo gate --no-fail-fast`: **533 passed, 1 failed, 1 pre-existing ignored**.
  The only failed target is `start_fox_600`, at the dynamics guard above.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- The gate ran the original idle replay, the separate state-callback replay,
  100k native-C airborne oracle and three new entry/landing unit tests with
  local data present; these were not asset skips.

## Changed files

All task changes are under `crates/melee-ft`; concurrent hsd-particle edits
were left untouched. No changes to TRACKER.md, CLAUDE.md, root Cargo.toml,
third_party, harness, melee-sim or traces; no commits.

- `src/fighter/entry.rs`, `fall.rs`, `landing.rs`, `effects.rs`: state logic,
  transitions and effect requests.
- `src/fighter/assets.rs`, `commands.rs`, `mod.rs`, `procs.rs`, `spawn.rs`,
  `state.rs`: resources, landing command, ownership and callback dispatch.
- `src/physics/airborne.rs`, `src/physics/mod.rs`: airborne arithmetic.
- `src/collision/air.rs`, `src/collision/mod.rs`: air/fixed-ECB map wrappers.
- `src/anim/playback.rs`: SM_None removal/frame behavior.
- `src/input/iasa.rs`: make the existing predicate evaluator crate-visible so
  Landing reuses the same predicates with its own ordering.
- `tests/fighter_support/mod.rs`: extend initial import for Entry/SM_None;
  retain the idle import and its existing active-script checks.
- `tests/start_fox_600.rs`: complete-proc acceptance test, currently failing.
- `tests/start_fox_states.rs`: independent state-callback integration test.
- `tests/airborne_ref_oracle.rs`, `tests/ref/airborne/driver.c`, `NOTICE`,
  `ftCommon_ApplyFrictionAir.c.inc`, `ftCommon_8007D174.c.inc`,
  `ftCommon_8007D28C.c.inc`, `ftCommon_Fall.c.inc`: C reference verification.
- `src/fighter/README.md`, `src/fighter/START_FOX.md`: status and this report.
