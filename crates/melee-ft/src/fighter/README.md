# Fighter ownership and the T10 idle path

Current dynamics work: the un-ignored full `start_fox_600` replay now passes.
The strict 130-tick match-start and 8-tick idle bone tests still fail; the
first match-start difference is tick 1, bone 17's unused `rotate[3]` word.
See [the dynamics report](../dynamics/README.md) for implementation, fusion
audit, exact differences and validation. The text below records the earlier
callback/idle milestone and its then-unimplemented dynamics boundary.

The match-start extension and its remaining full-proc blocker are documented
in [START_FOX.md](START_FOX.md). The T10 results below describe the original
idle gate; they do not certify the match-start dynamics path.

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
| `ftCo_Fall_Enter` | initial airborne Fall entry only; its ticking callbacks fail explicitly |
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

Non-idle IASA transitions, CPU AI, airborne ticks, unsupported motion entry,
teeter/fall from a ledge, sloped leg correction/body tilt, altered shield health,
active dynamic-bone solving and scaled-player attribute modifiers also fail
explicitly. Cold Fall creation is supported; advancing it to a landing is not.

The reachable Wait command subset executes timing, call/return, part animation
and ground-pose commands. Texture-frame commands retain requests for a renderer;
TObj/material/GX execution remains outside this simulation slice. Other opcodes
are rejected while loading. This is not a general T8 command interpreter.

## Verification and initial-state import

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

Final checks: `cargo gate` passed (514 passed, zero failures, one pre-existing
ignored doctest); `cargo clippy --workspace --all-targets -- -D warnings` passed.
The four cold-spawn/guard tests and the 600-record replay ran with local data.
No protected files, game data or commits were added.
