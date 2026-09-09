# M3 implementation plan: two idle Foxes on Yoshi's Story

Research snapshot: 2026-09-08. Scope is `harness/scenarios/idle_ys_fox.toml`, `Gr_Kind_Story`,
and the existing 600-record `idle_ys_fox.expected.jsonl`. FD is locked; the older FD tasks in
TRACKER are not this gate. No Cargo, Dolphin execution, implementation changes, or commits
were made for this plan. Unless prefixed otherwise, C paths below are relative to
`third_party/melee-decomp/src/melee/`; `hsd/` means
`third_party/melee-decomp/src/sysdolphin/baselib/`. Addresses are retail GALE01 symbols; line
counts include signatures, braces, comments and blanks.

## 1. Contract and the first blocker

The trace contains **49 scalar keys per record**, not 49 Fighter members: 24 per fighter, plus
`rng.seed`. Records are `{frame, phase:"frame_end", state}`; floats compare their `v.bits`,
never `v.approx`. Ordinal 0 is the loaded savestate itself, before advancing Rust. Initial
seed is 249636915 (retail `seed`, 0x804D5F90). Both motion IDs remain 14, kind 1, grounded,
zero damage, zero jumps, zero self/knockback velocity for all 600 records. Positions remain
(-42, 23.450098037719727, 0) and (+42, 23.450098037719727, 0), facing +1/-1. P0 starts at
animation time 6; P1 starts at 0. CPU timers remain 4/0.

**Do not yet equate one capture ordinal with one complete simulation tick.** The first 600
records were inspected, including the raw Fighter dumps:

| Evidence | Observation |
|---|---|
| P0 animation resets | ordinals 114, 234, 354, 474, **593** |
| P1 animation resets | 120, 240, 360, 480, **599** |
| P0 ordinals 485..488 | 11, 13, 13, 15; raw anim_id=2, rate=1 |
| P1 ordinals 495..498 | 15, 17, 17, 19; raw anim_id=3, rate=1 |
| Raw P1 animation | switches Wait1 row 2 → Wait2 row 3 at 480, back to 2 at 599 |
| Raw blending at ordinal 0 | P0 blend duration/progress=6/6; P1=6/1 |

`harness/dolphin/trace_scenario.py:161-172` merely names the frameadvance dump `frame_end`.
The locally installed Dolphin source actually emits FrameAdvance from `Core::OnFrameBegin`
(`~/Projects/dolphin-scripting/src/Source/Core/Core/Core.cpp:183-185`), called by VI
`BeginField` (`.../Core/HW/VideoInterface.cpp:825-827`), not a Melee GObj boundary. CPU-thread
synchronous reads do not make a VI boundary a game tick boundary. These observations are
consistent with sampling partway through different procs; they do not alone prove the cause of
each anomaly.

First capture PC, game tick, current proc, and RNG call sites at those ordinals. If they
confirm partial ticks, reproducing this exact file requires a defined VI-to-proc observation
schedule, potentially instruction timing beyond M3's headless loop. A separately captured
scheduler-end oracle would be useful but must not replace, normalize, or weaken this expected
file without resolving the contract. Treat the full gate as blocked on this investigation, not
green because an animation counter modulo 120 matches most rows.

## 2. Frame order to implement

Scene setup is `gm_Scene_Vs_OnEnter` → `fn_8016E730` (`gm/gm_16AE.c:1985-2040`): camera,
refraction, effects, player/common data, stage data, item system, stage initialization,
fighter spawn, stage start, HUD. `gm_801A4D34` (`gm/gm_1A45.c:265-374`) drains pad samples,
evaluates input, calls the scene's `on_frame` (`gm_Scene_Vs_OnFrame`, `gm_16AE.c:1527-1553`),
sets pause masks, calls the configured pre-proc callback, then `HSD_GObj_80390CFC` at line
340. Rendering follows the simulation loop. `gm_8016B0B0` is not the scheduler in this
revision.

**Sort by proc priority `s_link` first, then GObj `p_link`, then the GObj's `p_priority`/list
order.** `hsd/gobj.c:88-142` visits s_link 0..24 and masks paused p_links;
`hsd/gobjproc.c:12-103,161-179` inserts procs into those lists. `hsd/gobjplink.c:37-44,60-104`
appends equal-priority GObjs in creation order. Same-GObj, same-s_link registrations execute
in registration order. Preserve new-proc insertion, deferred deletion and visitation stamps,
already implemented by `hsd-gobj::World` (`crates/hsd-gobj/src/world.rs`). Do not run all of
P0's callbacks before P1's. Walk the fighter list; its order is the trace's p0/p1.

The following is the exact registered simulation spine; conditional item/effect instances and
pause flags must be inventoried from the savestate in task T0. Numbers are decimal. Fighter
entries execute P0 then P1 at that phase.

| s_link | p_link / p_priority | Callback, in order within phase |
|---:|---|---|
| 0 | 0/0; 3/0; 4/0; 8/0; 9/0 | scene `fn_801A4BD0` (empty); stage lights `Ground_801C461C`; `fn_801CADBC` stage spawn manager; `Fighter_8006A1BC` hitlag/status; existing `Item_802693E4` |
| 1 | 5/0; 8/0; 9/0 | `Ground_801C1CD0` for map IDs **0,1,3,2**; `Fighter_8006A360` animation/status; existing `Item_80269528` |
| 2 | 8/0 | `Fighter_8006ABA0`: CPU AI gate, false for both humans |
| 3 | 8/0 | `Fighter_Spaghetti_8006AD10`: pad parsing, buffers, Wait IASA |
| 4 | 5/0 | per map 0,1,3,2: `Ground_801C1D38` then registered stage proc (none, `grStory_801E322C`, `grStory_801E3334`, `grStory_801E33E0`, respectively) |
| 4 | 8/0; 9/0 | `Fighter_procUpdate` physics/integration; `Item_802697D4` physics/integration (including newly spawned items) |
| 5 | 9/0 | `Item_80269978` map collision |
| 6 | 8/0 | `Fighter_procMap`: Wait collision, update root translation |
| 7 | 8/0 | `Fighter_8006C5F4` → `ft_80089B08` ground pose/IK |
| 8 | 8/0 | `Fighter_CallAcessoryCallbacks_8006C624` |
| 9 | 8/0; 9/0 | `Fighter_8006C80C` effects flush/hitbox positions; `Item_80269A9C` |
| 10 | 5/0 | `Ground_801C0C2C` stage queries; Bob-omb rain branch disabled |
| 11 | 9/0 | `Item_80269B60` |
| 12 | 8/0; 9/0 | `Fighter_UnkProcessGrab_8006CA5C`; `Item_80269BE4` |
| 13 | 8/0; 9/0 | `Fighter_8006CB94` hit detection; `Item_80269C5C` |
| 14 | 8/0; 9/0 | `Fighter_ProcessHit_8006D1EC`; `Item_8026A294` |
| 15 | 11/1; 12/1 | `efLib_particles_proc_main`, then `efLib_particles_proc_aux`; per-effect `efLib_Update` uses its own p_link and p_priority 0 |
| 16 | 8/0; 9/0 | `Fighter_8006D9AC` dynamics; `Item_8026A788` |
| 17 | 15/0; 16/0 | HUD `ifStatus_802F5B48` then `ifStatus_802F4EDC` per HUD GObj; optional countdown; `ifTime_UpdateTimers` |
| 18 | 8/0; 18/0 | fighter `Fighter_UnkCallCameraCallback_8006D9EC` → `ftCamera_UpdateCameraBox`; camera `fn_8002F360` |
| 20 | 0/0 | `fn_8016C7D0` → `fn_80171DC4` match bookkeeping |
| 21 | 17/0 | `fn_8017C1A4` when its match mode is installed; verify VS instance |
| 22 | 8/0 | `Fighter_8006DA4C`: copy position/facing to Player and statistics |

Registration evidence: `ft/fighter.c:852,897-911`; `gr/ground.c:708-709,835,933-934,2750`;
`gr/grstory.c:79-82`; `gr/inlines.h:33-49` (adds a **separate** priority-4 proc);
`gr/grzakogenerator.c:313-328`; `it/item.c:957,991-1000`;
`ef/eflib.c:170-176,481,534,670-680`; `if/ifstatus.c:693-699`; `if/iftime.c:219,241-263`;
`cm/camera.c:4095-4111`; `gm/gm_1A45.c:232-234`, `gm/gm_16AE.c:2023-2027`, `gm/gm_17C0.c:262`.
The callback-table comments calling map 1 Randall and map 2 Shy Guys are misleading: the
**moving cloud collision/puff implementation is map 2**; the Shy Guy timer lives on map 3.
Follow the function bodies and GrJoint data.

## 3. RNG ledger: decorations are simulation dependencies

All 599 adjacent seed pairs were inverted by iterating the retail LCG `s = (214013*s +
2531011) mod 2^32` (`hsd/random.c:6-22`): draw counts `{0:183, 1:374, 2:13, 3:3, 4:20, 5:5,
7:1}`, **521 draws** total. Thus 416 transitions change seed; there is no unconditional
once-per-frame draw. `gekko-math` already implements the generator, not the consumer schedule.

Ordered consumers reachable on the idle path:

1. **s_link 1, fighter-list order:** `ftCo_Wait_Anim` → `ftCo_8008A7A8` →
   `getAnimID`, `ft/ftwaitanim.c:47-59,65-100`: Randi(100)+1 on animation end.
   PlFx's `ftData+24` table is `(2,70),(3,30),(-1,-1)`. Wait1 can repeat;
   Wait2 cannot repeat (`inlineA0`/retry loop, lines 39-45,77-79), so returning
   from Wait2 may consume several draws. Raw P1 selects row 3 at ordinal 480.
   Retail calls verified with `python3 -B harness/asm.py ftCo_8008A7A8 --calls`
   (Randi callsite 0x8008A8BC). Wait1/Wait2 subaction scripts were read from
   PlFx.dat: their reachable command streams contain no random-SFX opcode.
2. **s_link 4, map 3:** `grStory_801E3418`, `gr/grstory.c:233-286` only when
   no Heiho exists and the timer was already zero. In order: discarded timer
   Randi(1800) (`136,143-148`); Randi(6) until pattern differs (`257-258`);
   Randi(3) speed variant (`271`); Randi(8), and conditional Randi(3) (`157-162`);
   **then another** Randi(2), and conditional Randi(3) (second call at `277`);
   spawn each Heiho, then Randf vertical jitter (`227,283`—including the last
   unused jitter). The overwritten timer and first count still consume RNG.
   Count is 1 or **3..5**, despite the prose comment saying 3..6.
3. **Inside each Heiho spawn:** `it/itheiho` means `it/kinds/itheiho.c`:
   `it_802D8618:54-62` → `it/itzako.c:53-98` → item init →
   `it_802D8688:64-90` tries to create carried food. `it/kinds/itfoods.c:51-57`
   returns NULL when foods are disabled, before its random food choice at
   `itFoods_Logic18_Spawned:87`. Verify the live item-switch bits, not just
   an empty fighter held-item pointer. With foods off this consumes no draw.
4. **s_link 4, map 2:** `grStory_801E366C`, `gr/grstory.c:291-306` uses signed
   post-decrement: old timer >=0 returns. Otherwise queue puff `(bank=0,id=44)`
   through `grLib_801C97DC` (`gr/grlib.c:66-69`) → `hsd_8039F6CC`, then
   Randi(20)+10. The retail calls are 0x801E36A8 then 0x801E36B0. Consecutive
   puff ticks are separated by 12..31 ticks, not 10..29.
5. **s_link 15:** main particle interpreter then generator pass
   (`ef/eflib.c:670-674`), followed by aux links (`676-680`). Puff 44 uses
   `EfCoData.dat`, not GrSt's bank 30. Measured descriptor at archive data
   offset 0x2268: type=0, genLife=1, particle life=15, radius=0, angle=0,
   random=-1, kind=0 before runtime bank fixup. Emission consumes radius
   Randf (`hsd/generator.c:586`), angle Randf (`662`), even with zero radius
   and angle; immediate particle interpretation consumes Randf for opcode
   ED, random rotation (`hsd/particle.c:2746-2780`, specifically `2776`).
   This is three draws after the stage timer's one, with existing generators
   interleaved according to their linked-list order (`generator.c:311-349`).
6. **The sustained consumer:** `hsd_8039D3AC`, `hsd/generator.c:88-98`, cannot
   free a generator with live children, so sets `random=0`, `genLife=1`.
   `hsd_8039EE24:994-1008` still evaluates **0 * HSD_Randf()** at line 998.
   Each surviving puff generator therefore burns one draw per update,
   including its final attempted removal. Particle life and deletion happen
   first (`particle.c:2828-2871,2965-2995`). No rendering is required, but this
   lifecycle and the generator/particle list order are required.

Cross-check, not an independently captured call ledger: an inferred initial puff timer 15 and
eight remaining generator updates, the seven-draw Shy Guy event at ordinal 13, observed Wait
resets, and this puff lifecycle account for **every seed transition through ordinal 509**.
Inferred puff ticks begin
17,45,63,83,106,118,142,162,174,192,217,242,264,291,312,327,356,381,411,432,450,480. The next
predicted four-draw tick at 511 appears split into one draw at 510 and three at 511. This
supports investigating VI/proc timing; it is not license to hardcode per-ordinal draw counts.
Late residuals and complete active-instance ordering remain unverified until T0 records the
actual calls.

Other paths audited, and their exclusion conditions:

| Path | RNG consequence for this scenario |
|---|---|
| Human input | `ftCo_800A2040` (`ft/kinds/ftCommon/ftCo_0A01.c:1080-1089`) checks **Player slot kind**, not cpu.type. Both humans take pad branch `fighter.c:1818-1840`; AI proc at 1703-1709 is inactive. CPU initialization still draws once at `ftCo_0A01.c:730` (f64 `10.0 * Randf`, truncate), explaining frozen random timers 4/0. |
| Camera | `camera.c:3598-3650` draws only when entering `CAMERA_CLEAR`; normal `fn_8002F360:3457` dispatch does not call it. Normal camera/quake update has no direct RNG. Preserve stage bounds/subject data; full camera motion is not needed solely for seed. |
| Heiho hit states | Randf at `itheiho.c:201` and Randi(3) at `218` require hit/falling state 2. Unattacked flight states 0/1 do not draw. Keep motion/despawn logic so the next spawn timer starts correctly. |
| Other effects | `ef/efsync.c:242,257-258`, `ef/efasync.c:36,99,576-577` are effect-specific; no blanket no-op until active effects are inventoried. Additional particle bytecodes/generators can draw; puff's measured script only needs ED plus deterministic commands. |
| HUD / Player | `if/ifstatus.c:139-148,181-217` randomizes death and damage shake only; both fighters stay at zero percent. `pl/player.c` has no direct RNG. Percent derives from Fighter damage; stock count is not among the 49 keys. Prize RNG is outside this match phase. |
| Refraction / sound | `lb/lbrefract.c` has no RNG. `ft/ftaction.c:747-764` pseudo-random SFX is not in the two Wait scripts; `lb/lbaudio_ax.c:1288` randomizes a specific audio path, requiring runtime audit if reached. Audio cannot be discarded merely because it is inaudible headlessly. |
| Other stage paths | `gr/ground.c:514,559,564` Bob-omb-rain choice is gated at 674; crowd/SFX selection at 1365-1419 and 1476 is event-driven. `gr/grzakogenerator.c:89,95` belongs to configured spawn geometry, not unconditional `fn_801CADBC`; Story initializes the descriptor argument NULL at `grstory.c:91`. |

## 4. Fighter dependency order and port surface

Do not port names suggested by old address guesses: 0x800D5CB0 is
`ftCo_LandingFallSpecial_Enter`, not grounded Wait snap; `ftCommon_8007D5D4` **enters air**
and sets jumps_used=1 (`ftcommon.c:515-525`). Grounded Wait does not apply gravity every
frame. Port init/landing and grounded physics.

### (a) Data and spawned Fox

Read `PlCo.dat:ftLoadCommonData` common attributes and parts table (root slots 0 and 4,
`fighter.c:179-211`); `PlFx.dat:ftDataFox` common attrs +00, Fox attrs +04, parts +08, motion
rows +0C, blend bytes +10, Wait choices +24, dynamics +2C, hurtboxes +30, camera +3C, ECB
descriptor +44; +40/+50 for copied pickup attributes. Existing `melee-ft::desc` covers motion
rows/AJ subarchives only. `ftCo_DatAttrs` (`ft/types.h:686-772`) is 0x184 bytes; grounded
essentials are walk_max_vel +08=1.6, friction +18=0.08, ground max +34=3, max_jumps +58,
gravity +5C=0.23, terminal velocity +60=2.8, model_scaling +8C=0.96. Values above are display
decimals; load their actual f32 words. Keep player scale separate from model scale; ECB uses
player scale. `ftData+44` supplies six bone indices, center offset and ledge-snap dimensions
(`ft/types.h:584-595`, `ft_081B.c:33-68`), not a fixed guessed rectangle.

| Dependency order / function | C source span; lines | Retail |
|---|---|---|
| `Fighter_LoadCommonData` | ft/fighter.c:179-211; 33 | 80067ABC |
| `ftCo_800D0FA0`, then `ftCo_800D105C` | ft/ftchangeparam.c:148-157; 10 / 164-250; 87 | addresses in names |
| `ftFx_Init_OnLoad`, `ftFx_Init_LoadSpecialAttrs` | ft/kinds/ftFox/ftfox.c:486-501; 16 / 503-506; 4 | 800E57AC / 800E5858 |
| `ftParts_80074E58`, `ftParts_SetupParts` | ft/ftparts.c:673-693; 21 / 392-455; 64 | 80074E58 / 800743E0 |
| `ftCommon_GetModelScale`, `Fighter_UpdateModelScale` | ft/ftcommon.c:1407-1410; 4 / ft/fighter.c:213-230; 18 | 8007F694 / 80067BB4 |
| `ft_80081B38`, `ft_80082A68` | ft/ft_081B.c:33-68; 36 / 488-502; 15 | addresses in names |
| `ftCommon_UnlockECB`, `ftCommon_8007D6A4`, `ftCommon_8007D7FC` | ft/ftcommon.c:509-513; 5 / 546-563; 18 / 581-594; 14 | 8007D5BC / named / named |
| `ftColl_8007B320` hurtboxes; `ftCo_800A101C` CPU defaults | ft/ftcoll.c:3215-3254; 40 / ft/kinds/ftCommon/ftCo_0A01.c:647-830; 184 | named / named |
| `ftCommon_8007E2FC` + inline reset helper | ft/ftcommon.c:911-939; 29 | 8007E2FC |
| `Fighter_UnkInitLoad_80068914`, `Fighter_UnkInitReset_80067C98` | ft/fighter.c:683-808; 126 / 232-512; 281 | addresses in names |
| `Fighter_UnkProcessDeath_80068354`, `Fighter_Create` orchestration | ft/fighter.c:514-563; 50 / 846-931; 86 | 80068354 / 80068E98 |

Skeleton loading and basic bone lookup already exist in `hsd-anim`; runtime Fighter part
flags/masks, a second animation skeleton, ownership and ECB lookup glue do not.
`Fighter_Create` also initializes non-recorded gates; implement the reachable defaults
explicitly, not an assumed memset (C allocation is uncleared, `fighter.c:854-859`). Spawn
coordinates come through Player (`pl/player.c:228-240`, `gm/gm_16AE.c:1875`), then the ±10
vertical probe in `ft_80082A68` finds the platform before ground entry and Wait.

### (b) Wait, (c) grounded physics, (d) map collision

| Dependency order / function | C source span; lines | Retail |
|---|---|---|
| `ftCommon_ApplyFrictionGround` → `ftCommon_ApplyGroundMovement` | ft/ftcommon.c:51-59; 9 / 133-152; 20 | 8007C930 / 8007CB74 |
| `ft_GetGroundFrictionMultiplier` → `ft_80084F3C` | ft/ft_081B.c:1235-1241; 7 / ft/ft_084E.c:42-53; 12 | 80084A40 / 80084F3C |
| `ft_80084280` + its inline helper | ft/ft_081B.c:1062-1099; 38 | 80084280 |
| `ftCo_Wait_Phys`, `ftCo_Wait_Coll` | ft/kinds/ftCommon/ftCo_Wait.c:69-73; 5 / 75-78; 4 | 8008A644 / 8008A678 |
| `ftCo_8008A6D8`, `ftCo_8008A7A8` (+ helpers 39-60) | ft/ftwaitanim.c:19-37; 19 / 62-105; 44 | addresses in names |
| `ftCo_Wait_Anim`, `ftCo_Wait_IASA` | ft/kinds/ftCommon/ftCo_Wait.c:34-42; 9 / 44-67; 24 | 8008A494 / 8008A4D4 |
| `Fighter_ChangeMotionState` Wait branch | ft/fighter.c:933-1391; 459 (split task) | 800693AC |
| `ft_8008A348`, `ft_8008A2BC`, `ftCommon_8007D92C` | ft/ft_08A1.c:71-109; 39 / 54-63; 10 / ft/ftcommon.c:596-604; 9 | addresses in names |
| `ftCo_800A2040`; neutral input/buffers | ft/kinds/ftCommon/ftCo_0A01.c:1080-1089; 10 / ft/fighter.c:1777-2140; 364 (split) | 800A2040 / 8006AD10 |
| `Fighter_8006A1BC`, `Fighter_8006A360`, `Fighter_8006ABA0` | ft/fighter.c:1393-1442; 50 / 1444-1701; 258 / 1703-1709; 7 | addresses in names |
| `Fighter_procUpdate`, `Fighter_procMap` | ft/fighter.c:2150-2438; 289 / 2476-2516; 41 | 8006B82C / 8006C27C |
| `ft_80089B08`; `ftCo_800A0DA4` hurtbox extents | ft/ft_0899.c:73-236; 164 / ft/kinds/ftCommon/ftCo_0A01.c:553-609; 57 | addresses in names |
| `Fighter_8006DA4C` Player mirror | ft/fighter.c:3073-3083; 11 | address in name |

Wait IASA tests special/attack/grab/shield/taunt/jump/dash/squat/turn/walk predicates in the
order written at `ftCo_Wait.c:46-66`. Port the neutral predicate path with asserted
preconditions; never enter unsupported actions silently. Attack100, capture, status and
accessory gates in the proc wrappers also need initialized false/NULL defaults; a no-op must
have a reachability test.

Physics sequence: friction writes ground acceleration; ground movement projects velocity onto
floor tangent; procUpdate adds acceleration, anim velocity, nudge, self/knockback/shield
velocities, then moving-floor speed (`mpGetSpeed`, `fighter.c:2380-2390`) and wind. Preserve
signed zero and addition order. Wait collision goes **through ft_081B.c**, not a new algorithm
in ftcoll: copy last/current position and stick/facing → `mpColl_8004B4B0` → copy corrected
position back → teeter/fall only if support fails. `melee-mp` already ports this entry, spawn
`mpColl_800471F8`, JObj ECB fitting, geometry, floor normals, moving-joint transforms and
speed. Supply real bone positions and previous joint transforms
(`crates/melee-mp/src/mpcoll.rs:61,281,3221,3537`, `map.rs:561,823`). `ftColl_8007AEE0:3077`
only clears a shield collision flag; it is not ground snap.

### (e) Animation and commands

| Dependency order / function | ft/ source span; lines | Retail |
|---|---|---|
| `ftData_80085CD8` | ftdata.c:1729-1780; 52 | address in name |
| `ftAnim_8006F4C8`, `ftAnim_8006FCE4`, `ftAnim_8006FE08` | ftanim.c:594-636; 43 / 860-905; 46 / 907-914; 8 | addresses in names |
| `ftAnim_8006EBE8` attachment/request/blending | ftanim.c:388-428; 41 | address in name |
| `ftAnim_8006E7B8`, `ftAnim_8006F3DC`, `ftAnim_IsFramesRemaining` | ftanim.c:265-312; 48 / 552-571; 20 / 515-538; 24 | named / named / 8006F238 |
| `ftAnim_8006E9B4`, `ftAnim_8006EBA4` | ftanim.c:314-377; 64 / 380-386; 7 | addresses in names |
| `ftAnim_800707B0` part animation | ftanim.c:1118-1163; 46 | address in name |
| `ftAction_80073240`; `ftCo_8009E7B4` dynamics reset | ftaction.c:1318-1348; 31 / ftdynamics.c:604-704; 101 | addresses in names |

`melee-lb::anim` already attaches FigaTrees; `hsd-anim` already evaluates AObj, FObj, JObj
SRT/matrices and blending primitives. Fighter-level blend routing, part masks, command timing
and Wait reselection remain. `cur_anim_frame` is read from an eligible bone AObj or the blend
skeleton, not incremented directly. Wait1 row 2 has blend bytes 06/00, Wait2 row 3 has 00/00;
both flags=0x10000001. Wait1's script at data 0x4124 calls 0x40F4 at animation times 20,30,80;
these are deterministic part-animation commands (opcode 41), not empty scripts. Wait2 script
0x4150 changes part state. Port `lb/lbcommand.c:13-98` timer, call/return dispatch and the
reached ftAction handlers. `ftAnim_800707B0` (`ftanim.c:1118`) advances additional part
animations and cannot simply vanish. M2's two-frame unfiltered Wait1 proof does not prove
600-frame Fighter playback.

### (f) Ownership of all recorded fields

Offsets are hex relative to Fighter; vec3 expands to three scalar keys. `fighter.yaml` is the
actual subset; `fighter.generated.yaml` cross-checks the aliases below, not an instruction to
emit all 692 generated members.

| Trace suffix (per p0/p1) | Offset; C field | Writer on this path |
|---|---|---|
| kind, player_id | 004 s32, 00C u8 | `Fighter_UnkInitLoad`, fighter.c:688-689 |
| motion_id | 010 s32 | `Fighter_ChangeMotionState`; Wait entry/table ftmotionstates.c:288-299 |
| facing_dir | 02C f32 | `Fighter_UnkInitReset:239`; no neutral-input turn; mirrored to Player by 8006DA4C |
| self_vel.x/y/z | 080/084/088 f32 | `ftCommon_8007E2FC:911-939`; `ftCommon_ApplyGroundMovement:149-151`, `Fighter_procUpdate:2296` |
| kb_vel.x/y/z | 08C/090/094; x8c_kb_vel | `ftCommon_8007E2FC:924-926` and `procUpdate:2179-2230`; no applied hit/KB |
| cur_pos.x/y/z | 0B0/0B4/0B8 f32 | init/reset 232-252; spawn probe 82A68; procUpdate integration; Wait collision copies back at ft_081B.c:1072 |
| ground_or_air | 0E0 s32 | spawn support → `ftCommon_8007D6A4:552`; unchanged while supported |
| cur_anim_frame | 894 f32 | reset fighter.c:283; `ftAnim_8006E9B4:376`; restart also evaluates animation immediately |
| percent | 1830 f32; dmg.x1830_percent | reset fighter.c:304 from `Player_GetDamage` (player.c:1081); conditional damage/heal in 8006A360/TakeDamage are inactive |
| jumps_used | 1968 u8; x1968_jumpsUsed | reset fighter.c:299; ground entry ftcommon.c:554; remains zero |
| cpu.buttons, cpu.lstick_x/y | 1A88 u32, 1A8C/1A8D s8 | `ftCo_800A101C:747-750`; AI disabled so unchanged; human pad state is a different struct |
| cpu.type, cpu.level, cpu.behavior | 1A94/1A98/1AA0 s32; cpu.xC/level/x18 | `ftCo_800A101C:674-701`: 4,1,1; type 4 does not mean CPU slot |
| cpu.timer | 1B04 s32; cpu.x7C | `ftCo_800A101C:730-731`; increments only in AI path at ftCo_0A01.c:8614, inactive |
| rng.seed (global, once) | 804D5F90 u32 | ordered consumers in §3 |

## 5. Yoshi's Story assets and collision

Actual filename is **`harness/roms/files/GrSt.dat`**, 824,648 bytes, StageData path
`/GrSt.dat` (`grstory.c:56-71`). Measured archive data offsets: `coll_data=0x29294`,
`map_head=0x1C4`, `grGroundParam=0x2A3A0`, `yakumono_param=0x2A484`, `itemdata=0x2A47C`,
`map_ptcl=0x292C0`, `map_texg=0x29420`. Ground scale is **0.7**, at GroundParam+0 (not +4).
Use archive-relative links including relocated offset zero; particle banks add their own
bank-relative relocation (`hsd/particle.c:159-224`).

Collision-only loader: `melee-mp::desc::read_public_coll_data` → `CollMap::load(data, scale,
GrKind::Story)`. Actual geometry: 34 vertices at 0x28F64, 29 lines at 0x29074, two 40-byte
MapJoints at 0x29244; section (start,count) pairs floor=(0,7), ceiling=(0,0), right=(7,11),
left=(18,11), dynamic=(0,0). A zero dynamic-line section **does not mean no moving platform**.
Joint 0 owns cloud floor 0 and vertices 0..2; joint 1 owns the static remainder. Side-platform
floors 1 and 5 span x=-85..-40 and 40..85 at y=33.5 before scale; top platform is floor 4 at
y=60; main floors 2,3,6 include slants. The captured Foxes stand on the side platforms, not
the main slanted floor. Scaled y=23.45 plus the collision tolerance produces the recorded y;
preserve the algorithm and f32 rounding, do not substitute a decimal spawn height.

For moving collision load map_head's four 0x34-byte model records at 0x84, JObj/AnimJoint
trees, joint-index mapping/spawn-marker table, and spline at 0x154. Map 2 root=0x18388, joint
animation=0x187EC; GrJoint record at 0x70 is (collision joint 0, -1, bone 1).
`Ground_801C2ED0:1642-1672` binds the bone, updates its transform and snapshots prior
vertices. Every tick HSD animation runs at phase 1; map 2 `grStory_801E33E0:213-219` refreshes
collision via `Ground_801C2FE0:1676-1732` before fighter physics. Preserve previous/current
matrices, hidden flags and joint IDs, not just the cloud's visible location. Map 3 also calls
`Ground_801C2FE0` and `lb_800115F4` (`grstory.c:184-189`).

For RNG additionally load yakumono floats (600,1800,8; heights 30,45,60,75,90,0), Heiho
article data/animation through `itemdata`, common puff 44 in `EfCoData.dat`, and the initial
live particle/generator state. Stage textures, GX geometry, and the full camera renderer are
unnecessary for collision. Keep particle scripts and lifetimes despite their visual purpose.

## 6. Crates, bounded tasks, and mechanical gates

Use `melee-ft::{fighter,spawn,attributes,parts,animation,commands,input,
physics,collision,states::wait,snapshot}` with named `Fighter`, `MotionState`, `GroundMotion`,
`PlayerControl`, `AnimationPlayback`, `EnvironmentCollision`. Use `ft-fox::{attributes,init}`
for Fox-specific callbacks; `melee-gr::{desc, stage,collision,story::{cloud,shy_guys}}`;
`melee-it::heiho` for item ownership; add an HSD particle lifecycle module/crate and
`melee-ef` glue if the workspace does not yet provide them.
`melee-sim::{scenario,assets,initial_state,frame}` owns composition and an injected shared
RNG, not a cross-layer re-export facade. Keep descriptive Rust names, source symbols/addresses
in doc comments, enums and bitflags, small conceptual functions. Audit every float/FMA site
against retail asm; existing collision/math audits do not cover new glue arithmetic.

The following test names are **proposed**, not existing/passing gates. Each implementation
task requires its named tests under `cargo gate` and `cargo clippy --workspace --all-targets
-- -D warnings`; harness changes also require pytest and schema checks. Local asset tests skip
cleanly when absent; the actual M3 qualification must run with assets and must not skip.

| Task / dependencies | Bounded work | Mechanical acceptance |
|---|---|---|
| T0 observation contract; first | Harness proc/RNG breakpoints and initial-state exporter, no game mutation | `m3_capture_phase_contract`: 600 ordinals with PC, game tick, current s_link/GObj, 521 RNG draws reconciled in order; account for 486–498 and 510–522 before claiming a complete-tick oracle |
| T1 schema / T0 export | Owned snapshot and validated initial state, ~200 lines glue | `m3_initial_snapshot`: exactly 49 keys at ordinal 0, all bits; export hidden animation/ECB/stage/item/particle state separately |
| T2 static Story | ~150 lines ground load/descriptor glue plus existing mp reader | `story_collision_archive`: 34/29/2 counts, seven floor IDs, scale bits; `story_spawn_support`: both ±42 positions/ground flags match native C spawn probe |
| T3 cloud / T2 | ground bind/update ~125 C lines + cloud init/tick <100; split animation descriptor glue separately | `story_cloud_collision_600`: all transformed collision vertices, previous transforms, visibility and point-speed vs phase oracle for 600 ticks |
| T4 Fox data/parts | first five rows of init table, split descriptor reading from ~170 C lines init/parts | `fox_spawn_attributes`, `fox_ecb_bones`: exact attr words, 73-joint mapping, six ECB bone positions vs native-C/retail fixtures |
| T5 grounded leaves / T2,T4 | friction/projection/ground-entry/spawn probe <150 C lines | `grounded_physics_native`: signed zero, positive/negative friction, slants, moving floors; `idle_ground_fields_600`: both positions, all velocities, ground_or_air, jumps_used, percent |
| T6 animation attachment / T4 | ftData load + attachment helpers ~190 C lines | `fox_wait_attach_native`: Wait1 blend 6 and Wait2 blend 0, part masks/AObj flags/time; retain existing M2 bone gate without changing its expectations |
| T7 playback / T6 | ftAnim playback/time/remainder + Wait choice ~230 C lines | `fox_wait_playback_600`: both cur_anim_frame and hidden anim_id/blend state against phase oracle; native weighted-choice oracle verifies repeat retry counts |
| T8 commands / T6 | lbcommand subset + reached ftAction/part-animation helpers, split at 200–300 C lines | `fox_wait_commands_native`: compare script PC/timer/return stack and part flags at times 0,20,30,80,101 and all 600 captured samples |
| T9 human input / T1 | split fighter.c:1777-1940 and 1941-2140; neutral IASA predicates as a third small task | `human_idle_input_600`: all seven cpu fields per fighter unchanged, neutral actual input buffers; native predicate order test; assert no AI calls/RNG |
| T10 spawn orchestration / T4–T9 | reset 281 C lines; separately init/create/death wrapper ~262; split ChangeMotionState 459 into reset and animation/callback installation | `fox_spawn_native`: all 24 fields per fighter plus flags/ECB/callbacks; verify CPU init draw even for human; never run init RNG again after importing the saved boundary |
| T11 puff RNG / T0 | generator create/remove ~200 C lines; separately emit shape-0 subset and particle ED/life interpreter, each ≤300 relevant C lines | `randall_puff_native`: timer, emission, ED and zero-emission tail seed after each phase; overlapping puffs and deletion-frame draw; no per-frame seed lookup |
| T12 Heiho / T0,T2 | grstory spawn helpers ~100 C lines; separate itheiho spawn/state0/state1/despawn chunks ≤250 C lines; item scheduler glue separately | `story_shyguys_native`: exact spawn order/count/pattern/retry/timers and RNG; foods-off rejects carried food; 600-tick alive counts/positions against retail |
| T13 frame integration / T3,T7–T12 | proc wrappers in ≤300-C-line pieces, then small sim loop | `m3_proc_order`: native scheduler event list including same-tick item insertion; `idle_ys_fox_600`: exactly 600 records ×49 keys, zero bit mismatches including RNG, observation contract from T0 |

Cold spawning and importing this savestate are separate tests. The 49 output keys cannot
reconstruct command stacks, blend skeletons, floor history, stage timers, previous Shy Guy
pattern, live items, or particle ages. Export an owned initial-state fixture from the
already-owned savestate (local, uncommitted), then advance from it. Do not feed subsequent
expected records back into Rust. If T0 proves emulator-timed observations, specify and
separately approve the additional scope needed to reproduce those observations; do not bake
the late animation stalls or RNG splits into gameplay code.

## 7. Breakpoints and remaining risks

Following docs/ORACLE.md, record entry/exit for `HSD_GObj_80390CFC`, `Ground_801C1CD0`,
`Fighter_8006A360`/`ftAnim_8006EBA4`, `Fighter_8006ABA0`, `Fighter_Spaghetti_8006AD10`,
`grStory_801E3418`, `grStory_801E366C`, `Fighter_procUpdate`, `Ground_801C2FE0`,
`Fighter_procMap`/`mpColl_8004B4B0`, item physics/map, damage procs, both ef particle procs,
HUD, `fn_8002F360`, and `Fighter_8006DA4C`. For every HSD_Rand/HSD_Randf entry/exit record
caller LR, seed pointer/value, GObj/proc identity, item kind/state, game tick and VI ordinal.
Randi calls Rand; count the LCG mutation once. Break at caller returns where needed, not
merely the first instruction of a function with multiple exits.

Open risks: capture phase is unproven; P2's physical pad is not overridden by the current
script (`apply_inputs` only sets port 0); human/CPU slot kind must be exported independently
of cpu.type. Ground allocations leave the previous Shy Guy pattern uncleared. M2 covers two
frames and excludes stale cached bone matrices; it does not explain every part animation or
bone 67's reciprocal scale. Slanted-floor/ECB glue and new particle arithmetic still need
fusion audits. Normal camera has no direct draw but stage-bound/culling inputs affect item
lifetime, hence later RNG. The full live proc/effect inventory and the late-frame RNG ledger
require T0; this document does not pretend static call-graph reachability proves which
instances ran at each VI sample.
