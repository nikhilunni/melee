# Battlefield: idle and match start

Lane B1, 2026-09-09. `idle_bf_fox` and `start_bf_fox` each reproduce
600 ticks × 49 keys with zero divergences, including the shared RNG seed.
No capture, scenario, ROM, or decomp data was changed; Dolphin was not run.

## Resources and composition

`melee-sim::scene_stage` selects a small stage descriptor (archive reader,
filename, music row) and an owned stage callback enum. Scenario validation,
asset selection, collision loading, restoration and scheduler registration
use that selection. Shared fighter code contains no stage branches.

`melee-gr::desc::read_battlefield` reads `GrNBa.dat`'s `map_head`,
`grGroundParam`, and `yakumono_param`; `load_collision` reads `coll_data`
through `melee-mp`. The archive contains seven models and section counts
`[1, 7, 0, 34, 0, 4]`. Map scale is the retail float `0.8`. All seven
model collision-binding lists are empty: the three soft platforms and main
floor use the existing static collision implementation. Sweeps verify main
floor y=0 (line 1), top y=54.4 (line 3), and side platforms y=27.2
(lines 2 and 4); only the three platform lines have the soft flag `0x100`.

Map 0's 21 position bindings include the two-player markers. P1's marker
is world (0,8); P2's is (0,62.4), eight units above the top platform.
Thus the note about y=8 is a height **above each platform**, rather than
both markers having world y=8. Imported idle positions are (0,0) and
(0,54.4); no FD-style ±60 spawn constants are introduced. The recorded
P1 start sequence agrees with the supplied notes: EntryStart 322 at tick 0,
Entry 323 at 6, EntryEnd 324 at 35, Fall 29 at 65, Landing 42 at 74,
Wait 14 at 104.

Map 6 supplies an ambient light (`0x808080FF`) and directional light
(`0xFFFFFFFF`). Both LightLists have null animation tables; neither has a
position constraint or linked successor. `hsd-archive::desc::light` owns
the HSD layouts; `melee-gr::battle::lights` loads and scales the directional
position through `Ground_801C466C` (0x801C466C). Its registered light update
therefore has no animation work or RNG. Map 6 has no fog descriptor.
Material/texture animation and GX display state remain rendering work.

Battlefield's StageParam row is stage-select ID 31: primary track 81,
alternate 38, chance 12, unlock rule 6. The captured all-characters unlock
mask enables the existing match-start `HSD_Randi(100)` call.

## Procs and boundary restoration

`grBattle_OnInit` (0x80219CA4) creates maps **0,3,1,6**. Their order matters:

| s_link | p_link | Work |
|---:|---:|---|
| 0 | 3 | `Ground_801C461C` (0x801C461C), static lights |
| 0 | 4 | `fn_801CADBC` (0x801CADBC), inactive spawn manager |
| 1 | 5 | `Ground_801C1CD0` (0x801C1CD0), each map in creation order |
| 4 | 5 | `Ground_801C1D38`, then each map's Battlefield callback |
| 10 | 5 | `Ground_801C0C2C` (0x801C0C2C), rain disabled |
| 15 | 11,12 | Shared effect and particle procs |

Map callbacks are 0x8021A114 (map 0, empty), 0x8021A3BC (map 3,
background controller), 0x8021A26C (map 1, empty), and 0x8021A174
(map 6, collision refresh and quake update). Empty collision-binding lists
and an empty saved `lb_804D63B0` quake list are checked during restoration.

`grAnime_801C8138` (0x801C8138) sets loop flags from the model's animation
flag table and immediately evaluates frame zero. Battlefield is already
past this evaluation at its start savestate: four background generators
exist; idle has six. Restore imports the complete initial population and
attachment matrices, map 3's signed timer, and model animation frames.
It rebuilds keyframe cursors from archive animations, discarding historical
spawn events. No future oracle rows enter the simulation.

The idle saved map-1 animation is at frame 238 of 400; map 6 is at frame
38 of 600. Start has both at frame 0. Generator attachment identity is map 1,
archive descendant 2, excluding Ground's wrapper joint. The wrapper applies
map scale 0.8 through the audited HSD matrix routines. Animation DPtcl
commands spawn bank-30 generators, and live attachment matrices are refreshed
before particle execution. Fighter entry and landing effects continue to
use the shared effect layer and bank 0.

## RNG audit

`harness/rng_ledger_report.py` reports 1,735 idle draws and 5,744 start draws.
The idle total is 11 Wait-animation choices plus 1,724 particle draws.
Start is eight Wait choices, six landing-effect placement draws, one
alternate-music draw, and 5,729 particle draws. The gate reproduces the full
stream; the added `m4_gate` tests also compare particle callsites in order.

The direct Battlefield controller draws at initialization and background
changes, outside these captured intervals:

- `grBattle_BG_Callback0` (0x8021A344), grbattle.c:333/353:
  `HSD_Randi(1200) + 2400`. Saved remaining timers are 2,490 (idle)
  and 3,128 (start); restoration does not re-consume initialization draws.
- `grBattle_BG_Callback2` (0x8021A3BC), grbattle.c:393-397:
  choose from map IDs `[1,2,4]` using `HSD_Randi(3)`, retrying the previous
  background. Timer reset at lines 419-422 uses the same 1200/2400 rule.
- Neither `grBattle_*` nor a Ground per-tick proc appears in either ledger.
  Start's `Ground_801C24F8+0x1B4` is the one pending music choice.

The particle runtime already implemented the required emission/randomization
branches. Battlefield additionally requires bytecode `0xB3`
(`hsd_8039930C`, 0x8039930C, particle.c:1502-1547): materialize the old alpha
comparison interpolation with signed 16.16 integer arithmetic, then load
its timer, mode and two targets. A behavior test covers restarting an active
interpolation and reaching both targets.

`asm.py --fused` was run for `grBattle_BG_Callback0`,
`grBattle_BG_Callback2`, `grBattle_GObj6_Callback2`, `Ground_GetStageGObj`,
`Ground_801C466C`, and `hsd_8039930C`. The new controller and alpha opcode
have integer arithmetic only; light position scaling has separate fmuls,
with no multiply-add. Animation evaluation and matrix products use the
existing audited HSD routines.

### Particle callsite counts

| Caller + call offset | Idle | Start |
|---|---:|---:|
| `hsd_8039930C+0x1504` | 0 | 111 |
| `hsd_8039930C+0x1D7C` | 116 | 581 |
| `hsd_8039930C+0x1DE8` | 116 | 581 |
| `hsd_8039930C+0x1E54` | 116 | 581 |
| `hsd_8039930C+0x1EC0` | 116 | 581 |
| `hsd_8039930C+0x1FE4` | 0 | 34 |
| `hsd_8039930C+0x2054` | 0 | 34 |
| `hsd_8039930C+0x20C8` | 0 | 34 |
| `hsd_8039930C+0x213C` | 0 | 34 |
| `hsd_8039930C+0x21F0` | 0 | 38 |
| `hsd_8039930C+0x22D4` | 0 | 20 |
| `hsd_8039930C+0x281C` | 0 | 34 |
| `hsd_8039930C+0x28D8` | 0 | 34 |
| `hsd_8039930C+0x2994` | 0 | 34 |
| `hsd_8039930C+0x2A50` | 0 | 34 |
| `hsd_8039930C+0x31EC` | 0 | 39 |
| `hsd_8039930C+0x3280` | 0 | 39 |
| `hsd_8039DAD4+0x1030` | 0 | 154 |
| `hsd_8039DAD4+0x1088` | 0 | 154 |
| `hsd_8039DAD4+0x10F8` | 0 | 154 |
| `hsd_8039DAD4+0x5B4` | 0 | 4 |
| `hsd_8039DAD4+0x710` | 0 | 119 |
| `hsd_8039DAD4+0x900` | 232 | 393 |
| `hsd_8039EE24+0xDC` | 1028 | 1746 |
| `hsd_8039F05C+0x1F4` | 0 | 162 |

## Verification and remaining branches

The shared `hsd-particle/tests/support/dust_replay.rs` replays both scenes,
using externally requested spawns and attachment matrices logged from the
production simulator. It compares all captured generator, particle and
AppSRT fields and ordered particle RNG sites:

| Scene | Ticks | Fields | Particle RNG draws | Excluded fields |
|---|---:|---:|---:|---:|
| idle_bf_fox | 600 | 3,483,282 | 1,724 | 0 |
| start_bf_fox | 600 | 2,925,912 | 5,729 | 0 |

The helper retains its documented AppSRT display-cache exclusion; neither
Battlefield capture needs it. Fixture provenance and regeneration are in
`crates/hsd-particle/tests/data/README.md`. Recorded outputs, expected bit
patterns, and comparison rules were not loosened.

Beyond these scenarios, transition animation attachment, material overlay
completion and background retirement remain explicit `unimplemented!`
branches with grbattle.c lines. Demo/event map 5, dynamic quake populations,
animated collision bindings, and non-static light lists are outside the
restored boundary and rejected. The timer and retry-selection primitives
are ported and tested; the complete multi-minute background cycle is not
claimed as an integrated scene. Rendering, camera-dependent display caches,
and audiovisual output remain outside the headless simulation scope.

Final workspace validation: `cargo gate` passed 655 tests, with zero failures
and one pre-existing ignored doctest. Its `m4_gate` suite passed all 69 tests;
all `hsd-particle` tests and all four `start_fox_bones_130` tests passed.
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all`,
and `git diff --check` passed. The opcode capability inventory was updated
for B3, retaining explicit truncated-bytecode checks and adding interpolation
behavior coverage; no retail oracle values were edited. Work remains
uncommitted, as requested.

## Changed files

- `crates/hsd-archive/src/desc.rs`, `src/desc/light.rs`: typed static light data.
- `crates/melee-gr/src/battle/{mod.rs,procs.rs,lights.rs}`,
  `src/{desc.rs,lib.rs,last/animation.rs}`, `tests/real_battlefield.rs`:
  controller, lights, shared archive/animation plumbing, collision checks.
- `crates/melee-sim/src/{scene_stage.rs,assets.rs,scenario.rs,frame.rs,lib.rs}`,
  `src/initial_state/{mod.rs,stage.rs}`, `tests/m4_gate.rs`: stage composition,
  restoration and regression gates.
- `crates/hsd-particle/src/particle.rs`, `tests/{lifecycle.rs,opcodes.rs}`,
  `tests/live_bf_{idle,start}.rs`, `tests/support/dust_replay.rs`,
  `tests/data/{idle_bf_spawns.json,start_bf_spawns.json,README.md}`:
  alpha compare behavior and particle replay coverage.
- `docs/BATTLEFIELD.md`, `TRACKER.md`: evidence, scope and session status.

## Lane B2: platform movement (2026-09-09)

`platform_bf_fox` passes **300 ticks × 49 keys, zero divergences**. It is now
in `m4_gate`, alongside an ordered particle-RNG ledger comparison. The
existing `melee-mp` one-way line handling and fighter Jump/Fall collision
callbacks already support rising through soft platforms and landing on them.
No collision geometry or expected values were changed.

The missing state was `Pass` (244, submotion 209), entered from Squat's
existing platform-drop timer. `ftCo_8009A228` leaves ground, clamps air drift,
loads the PlCo +0x46C downward velocity, attaches Pass, then sets
`CollData.floor_skip` to the supporting line and resets the stick timer to
0xFE. Its collision callback uses `ft_CheckGroundAndLedge`'s ordinary airborne
path, which respects that skipped line. Its physics uses ordinary fall
physics without introducing a new fast-fall check. The Pass animation ends
in Fall. The existing motion-change reset clears the skipped line on the
next transition (including Landing); there is no separate guessed timeout.
Pass's second motion scratch word is retained through landing.

The gate covers KneeBend at tick 30, the rising JumpF at 33, main-floor
Landing at 68, the second hop at 200/203, soft-platform Landing at 228,
Wait there at 258, Squat at 262, **Pass at 265**, and main-floor Landing at
280. P2 remains idle on the top platform throughout. `asm.py --fused` was
checked for Pass entry, drift clamping and Pass physics; the new entry path
has no multiply-add sites. No Dolphin run or new capture was necessary.

See `docs/YOSHIS_STORY.md` for the complete B2 file list and validation scope.
