# Fighter states and scheduler

Fox on Final Destination now supports the M4-T1 grounded movement slice:
Squat/SquatWait/SquatRv, standing Turn, and WalkSlow/Middle/Fast. The three
recorded 300-tick movement gates match all 49 keys, including the scene RNG.
Match-start Entry/Fall/Landing and the dynamic-bone solver are also implemented;
the older unimplemented-dynamics claims in START_FOX.md describe its original
callback-only milestone, not the current implementation.

Final M4-T1 validation: `cargo gate` **550 passed, zero failures, one pre-existing ignored doctest**; clippy clean. [Exact commands, file list and limits](M4_FOX.md).

## M4-T1 ports

Addresses below were resolved against the retail symbol map. State callbacks
are enums in `state.rs`; scratch data uses typed `MotionData` variants.

| Retail function(s) | Address(es) | Rust owner |
|---|---|---|
| `ftCo_Squat_Enter`, `ftCo_Squat_Anim`, `ftCo_Squat_IASA` | 800D600C, 800D607C, 800D60B8 | `squat.rs` |
| `ftCo_800D638C`, `ftCo_SquatWait_Anim`, `ftCo_SquatWait_IASA` | 800D638C, 800D6448, 800D6474 | `squat.rs`, shared animation restart |
| `ftCo_SquatRv_CheckInput`, `ftCo_SquatRv_Enter`, `ftCo_SquatRv_Anim`, `ftCo_SquatRv_IASA` | 800D65D8, 800D6620, 800D6658, 800D6694 | `squat.rs` |
| Squat / SquatWait / SquatRv Phys | 800D623C / 800D6584 / 800D6784 | shared `physics/grounded.rs`; only hold invalidates shield position |
| Squat / SquatWait / SquatRv Coll | 800D625C / 800D65B8 / 800D67A4 | `collision/ground.rs::map_ground_action` |
| `ftCo_Turn_Enter`, `ftCo_Turn_Enter_Basic`, `ftCo_Turn_Enter_Smash` | 800C9840, 800C98AC, 800C9C74 | `turn.rs` |
| `ftCo_Turn_Anim_Inner`, Anim, IASA, Phys, Coll | 800C9924, 800C9970, 800C99F8, 800C9BEC, 800C9C0C | `turn.rs`, shared ground helpers |
| `fn_800C9C2C` | 800C9C2C | Turn dash-direction buffering |
| `ftCo_Walk_CheckInput`, Enter, Anim, IASA, Phys, Coll | 800C9468, 800C9528, 800C95F4, 800C9614, 800C9768, 800C9788 | `walk.rs`, shared ground helpers |
| `ftWalkCommon_GetWalkType_800DFBF8`, `800DFCA4`, `800DFDDC`, `800DFEC8` | 800DFBF8, 800DFCA4, 800DFDDC, 800DFEC8 | walk tier, entry, rate and phase conversion |
| `ftWalkCommon_800E0060`, `ftCommon_8007C98C`, `ftCommon_ApplyGroundMovement` | 800E0060, 8007C98C, 8007CB74 | `physics/grounded.rs` acceleration, target clamp and projection |
| `ft_80084F3C`, `ftColl_8007AEE0` | 80084F3C, 8007AEE0 | shared friction, explicit shield-cache invalidation |
| `ft_80083F88`, `ft_80082708`, `ft_80084280` | 80083F88, 80082708, 80084280 | ordinary Squat/Turn collision versus Walk/Wait teeter collision |
| `ftCo_Wait_IASA`, `ft_8008A244`, `ft_8008A348` | 8008A4D4, 8008A244, 8008A348 | ordered movement entries and return to Wait |
| `Command_07`, `Command_08`, `ftAction_80073354` | 80005AE4, 80005B00, 80073354 | script goto, animation-loop wait and nonzero-phase seek |
| `ftAction_80072CD8`, `ftAction_800728F8` | 80072CD8, 800728F8 | FD footstep sound and controller rumble requests |

WalkFast shares the same selection, rate and physics implementation. Its tier
threshold boundaries are unit-tested; the supplied retail traces exercise only
Slow and Middle. Walk phase conversion uses retail `800E0010 fnmsubs`, followed
by separate divide/multiply and `fctiwz`. Acceleration uses separate `fmuls`
at 800E008C/0090/009C or 00AC, then `800E00B4 fadds`. No FMA is introduced there.
Every imported resource comes from the owned archives; gameplay reads no trace
or tick number.

`movement_fox_states` uses the common replay helper with trace-derived length
and an explicit `ledger` suffix. It supplies only recorded pad inputs and the
external pre-draw seeds needed to isolate fighter RNG. It compares 48 snapshot
keys plus raw submotion, rate, remainder, ground velocity, command clocks,
nametag timer and relevant Turn/Walk/Squat scratch fields. `m4_gate` separately
runs the complete scene and checks all 49 keys with produced RNG. Local-data
absence skips both test families cleanly.

Remaining explicit movement boundaries: TurnRun (ftCo_TurnRun.c:35-38), Turn to
Dash (ftCo_Turn.c:139-144), platform-drop entry (ftCo_Squat.c:79-84), ledge
Fall/Ottotto entries, attack/special/jump/shield/dash transition bodies, metal
or scaled-player modifiers, and non-default terrain footstep effects. The new
IASAs retain their own predicate ordering and reject unsupported transition
bodies. Sound/rumble are queued output requests; audio/controller playback is
outside the headless simulation. FD's default footstep mapping has no graphics
request or RNG. There were no new RNG sites or unexpected recorded transitions.

`Fighter<C>` owns `FighterPhysics` (including position and facing),
`FighterAnimation`, `FighterInput`, `EnvironmentCollision`, attributes, bone
descriptors, the main `JObjTree`, installed `MotionState` callbacks, command
state, CPU state, character hooks, and collision/camera caches. Source names
and Fighter offsets are documented on the fields. `FighterAssets` owns the
archive-derived resources; neither archive addresses nor trace ticks appear in
the gameplay state machine.

`Snapshot` emits exactly the 24 keys from `harness/schema/fighter.yaml`, including
all seven CPU fields for human slots. The scene wraps it in `PrefixSink` with
`p0` or `p1` and owns the separate global RNG field.

## Spawn and motion entry

The port composes these ordinary Fox paths, with exceptional interactions gated:

| C entry | Rust owner / behavior |
| --- | --- |
| `Fighter_Create`, `0x80068E98` | `Fighter::spawn`; caller supplies a costume skeleton, player slot and stage marker |
| `Fighter_UnkInitLoad_80068914`, `0x80068914` | `prepare`; typed data, parts, skeleton, input and character load hook |
| `Fighter_UnkInitReset_80067C98`, `0x80067C98` | spawn position/facing, velocities, damage, counters, shield and inactive interactions |
| `Fighter_NewSpawn_80068E40`, `0x80068E40` | shared `SpawnCounter`, increment with zero skipped on wrap |
| `Fighter_UnkProcessDeath_80068354`, `0x80068354` | initial support probe, model scale, character reset, CPU init and thrown capsule initialization |
| `ft_80082A68`, `0x80082A68` | vertical support probe through T5/MP |
| `ftCommon_8007D5D4`, `0x8007D5D4` | failed probe: airborne, one jump used, ten-frame ECB lock |
| `ftCo_800A101C`, `0x800A101C` | initializes CPU fields and consumes one RNG draw even for a human slot |
| `Fighter_ChangeMotionState`, `0x800693AC` | `change_motion_state`; reset motion-owned state, install callbacks, animation, command entry and dynamic-bone gates |
| `ft_8008A348`, `0x8008A348` | grounded Wait entry and nametag duration |
| `ftCo_Fall_Enter` | neutral airborne Fall entry and callbacks, including Landing |
| `ftLib_800867E8`, `0x800867E8` | clear/freeze input at the end of cold creation |
| `ftFx_Init_OnLoad`, `0x800E57AC` | `ft_fox::init::Fox`: owned special attributes, walljump/special capabilities and three item resource registrations |
| `ftFx_Init_OnDeath`, `0x800E5554` | clear blaster state and select default model group |

The reset assertions cover all 24 snapshot fields, previous position, spawn
number, input freeze, disabled/interaction state, hit/smash timer sentinels,
shield health, ledge cooldown, sword trail, walljump, Fox item registrations and
blaster state, thrown capsule state, dynamic-bone selection, callback table,
ground-pose bits, ECB lock, floor index, six ECB bones and model scales. This is
an owned idle representation, not a byte-layout port of every Fighter member.

Root scale is `player.scale * co_attrs.model_scaling` (`ftCommon_GetModelScale`,
retail `0x8007F69C`, separate `fmuls`), giving Fox `0.96` (`0x3F75C28F`). Bone 67
gets the reciprocal (`0x3F855556`) in T7's `reset_pose`, corresponding to
`ftCommon_8007F6A4`. Spawn X uses the retail `fmadds` at `0x80067CE8`. CPU init
uses double multiply/truncation at `0x800A124C`/`0x800A1258`. Other newly ported
arithmetic sites cite their checked instruction sequence beside the expression.

FD's stage-position bindings 0 and 1 are `(-60,10,0)` and `(60,10,0)`.
The retail probe does not find support there, so cold creation enters Fall.
The idle savestate is already grounded at approximately y=0.0001 and must be
imported separately. The original self-authored cold expectation assumed a
ground-level marker; it was corrected against the stage data and C path.
No oracle expectations were changed.

## Scheduler interface

`interleaved_order(2)` visits P0 then P1 within each row, preserving scheduler
link order. The scene supplies pad samples, wind, collision map, camera zoom
and shared RNG, and inserts its own stage/particle callbacks between phases.

| s_link | Method | C symbol |
| --- | --- | --- |
| 0 | `proc_status` | `Fighter_8006A1BC` |
| 1 | `proc_anim` | `Fighter_8006A360` |
| 2 | `proc_cpu_gate` | `Fighter_8006ABA0` |
| 3 | `proc_input` | `Fighter_Spaghetti_8006AD10` |
| 4 | `proc_update` | `Fighter_procUpdate` (`0x8006B82C`) |
| 6 | `proc_map` | `Fighter_procMap` (`0x8006C27C`) |
| 7 | `proc_pose` | `Fighter_8006C5F4` |
| 8 | `proc_accessories` | `Fighter_CallAcessoryCallbacks_8006C624` |
| 9 | `proc_hitbox_positions` | `Fighter_8006C80C` |
| 12 | `proc_grab` | `Fighter_UnkProcessGrab_8006CA5C` |
| 13 | `proc_hit_detection` | `Fighter_8006CB94` |
| 14 | `proc_process_hit` | `Fighter_ProcessHit_8006D1EC` |
| 16 | `proc_dynamics` | `Fighter_8006D9AC` |
| 18 | `proc_camera` | `Fighter_UnkCallCameraCallback_8006D9EC` |
| 22 | `proc_player_mirror` | `Fighter_8006DA4C` |

Animation, input, physics and map dispatch consult the installed callback set.
The ordinary attacks are disabled during Wait, but the thrown capsule still
advances its current/previous bone position at link 9 (`ft_8007C224`). Hurtbox
caches are invalidated at link 4 and feed CPU extents at link 14. Dynamic
collider positions update at link 16. Wait flags select disabled dynamic-bone
solving: `ftCo_8009CB40(..., false, NULL)` sets the first bone to `0x100`, and
`lb_8001044C` returns on that sentinel. This is checked against every ledger row.

## Explicit boundaries

`Status::interaction` is the scene's interaction boundary. A caller must mark
hitlag, items, grabs, shields, damage, death, status effects, accessories, active
attacks, queued effects, stage hazards, fighter overlap or coin-match rules
before dispatching such a scenario. Each arm has a C-located `unimplemented!`.
The isolated fighter does not scan other fighters/items or implement the scene's
collision registries. Empty ordinary-Wait branches are checked against the raw
ledger's attack, item, accessory, async and catch fields.

IASA bodies outside the movement slice above, CPU AI, unsupported motion entry,
teeter/fall from a ledge, sloped leg correction/body tilt, altered shield health,
and scaled-player attribute modifiers fail explicitly. Neutral Fall through
Landing/Wait and the Fox tail solver are implemented and covered by match-start
gates.

The reachable Wait command subset executes timing, call/return, part animation
and ground-pose commands. Texture-frame commands retain requests for a renderer;
TObj/material/GX execution remains outside this simulation slice. Other opcodes
are rejected while loading. This is not a general T8 command interpreter.

## Original idle verification and initial-state import

`idle_fox_600` compares all 48 fighter values through `Snapshot` and `melee-diff`:
P0 600/600, P1 600/600, first mismatch none. The fixture contains frame 0 and 599
transitions. All nine Wait RNG draws match. Particle draws are represented by
feeding each fighter draw's preceding seed from the ledger, as in T7; this does
not certify the standalone scene RNG stream.

The test imports frame-zero physics, input histories, command state, CPU fields
and complete CollData history. A test-only LZ4 reader also imports the main/blend
joint transforms, flags and matrix caches from the locally owned savestate.
It validates that its fighter and animation scalars match the ledger boundary.
An inferred Entry pose initially failed P1's capsule check at tick 1; importing
the actual saved pose fixed it. Both capsule positions, command timers/frames,
nametag countdown and dynamic-bone sentinel are additionally checked every tick.
No subsequent expected row drives gameplay, except the explicitly scoped RNG
seed input. The test skips if local disc files, traces or savestate are absent.

`fox_spawn_native` loads fresh costume skeletons and actual FD spawn markers.
Its reset expectations come from C stores/data and the cited retail arithmetic;
it does not claim a complete native-C clone of Fighter_Create. Other tests cover
CPU initialization modes, counter wrap, callback dispatch, unsupported-path
panics, the Wait callback table and scheduler interleaving.

Changed files (crate paths are relative to `crates/`):

- `melee-ft/src/fighter/{mod,assets,caches,commands,procs,snapshot,spawn,state}.rs`
- `melee-ft/src/fighter/README.md`
- `melee-ft/src/lib.rs` and `melee-ft/Cargo.toml`
- `ft-fox/src/{lib,init}.rs`
- `melee-ft/tests/{idle_fox_600,fox_spawn_native}.rs`
- `melee-ft/tests/fighter_support/{mod,collision,saved_pose}.rs`
- Root `Cargo.lock`: two local dev-dependency edges.

Original T10 checks: `cargo gate` passed (514 passed, zero failures, one pre-existing
ignored doctest); `cargo clippy --workspace --all-targets -- -D warnings` passed.
The four cold-spawn/guard tests and the 600-record replay ran with local data.
No protected files, game data or commits were added.
