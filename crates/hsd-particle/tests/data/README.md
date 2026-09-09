# M4 effect input fixtures

The six `*_fd_spawns.json` files were logged on 2026-09-09 from the production
`melee-sim gate harness/scenarios/<scene>_fd_fox.toml` runs at checkout `a5a45fd`
(including M4-T4 `c9a58af` and M4-T5 `78ef5e1`). All six gates reported zero divergences. This is
the same temporary-instrumentation method as `support/jump_fd_spawns.rs`.
No retail particle output was used to construct these fixtures.

To reproduce, temporarily log the following calls, run each gate, then remove
the logging:

- Label events with `Simulation::tick`'s frame before `world.run_procs()`.
- Log `SpawnRequest` immediately before each `ParticleSystem::spawn` call in
  `Effects::flush`, `Effect::animate`, and `Effects::spawn_dust_generator`.
  Retain bank, kind (`spawn`), link, position and optional joint ID/matrix.
  Assert that velocity and application-transform overrides are absent and
  mirroring is false; those paths are not used by these six scenes.
- Log `ParticleSystem::update_joint` and `ParticleSystem::expire_joint` calls.
  Keep only joints referenced by the external spawn requests for that scene.
- After the shield spawn in `Effect::animate`, log the caller's flag operation
  as `flags_joint`, `clear` (0x600), and `set` (0x1800).
- Store every float with `f32::to_bits`, group events by tick, and preserve
  their call order, including multiple updates to a joint within one tick.

The fixture replay applies those calls before the particle main/aux procs,
just as the production scheduler does. Children are spawned by particle
bytecode, never logged as external requests. There are no particle positions,
velocities, lifetimes, RNG results or compared generator/AppSRT values in the
fixtures. Position and matrix words are only the effect layer's inputs.

| Scene | Ticks | External spawns | Retained input events |
|---|---:|---:|---:|
| shield | 300 | 1 | 12 |
| spotdodge | 300 | 3 | 21 |
| roll | 300 | 4 | 43 |
| airdodge | 300 | 4 | 69 |
| wavedash | 300 | 3 | 29 |
| ledge | 420 | 4 | 64 |

## Battlefield (Lane B1, 2026-09-09)

`idle_bf_spawns.json` and `start_bf_spawns.json` were recorded from production
`melee-sim gate` runs of the corresponding Battlefield scenarios, each reporting
600 ticks × 49 keys with zero divergences. They use the same format and method
above, additionally logging map animation spawns and joint updates in `frame.rs`.
The temporary logging was removed afterward. No particle outputs were used as
input fixtures. Duplicate unchanged joint matrices and unused joint updates are
omitted; call order of all retained events is preserved.

| Scene | External spawns | Retained input events |
|---|---:|---:|
| idle_bf_fox | 8 | 9 |
| start_bf_fox | 12 | 259 |

The initial generators all attach to production joint ID 102: map 1, archive
descendant 2, after excluding Ground's scale wrapper. The helper resolves that
initial identity before consuming the logged events. Start tick 0 performs the
pending music choice but no scheduler procs, as in the saved boundary. Neither
Battlefield replay needs an AppSRT display-cache exclusion.

## M5 jab input fixture

`jab_fd_marth_spawns.json` was logged on 2026-09-09 from the Lane A1 production
jab gate, which reported 300 ticks, 49 keys and zero divergences. It has nine
external spawns and 41 retained input events. Use the procedure above, including
the spawn in `effects/dust.rs`. Directional run/brake dust also supplies an
AppSRT override: log its translation, rotation, scale and status plus the
request's mirror flag. These are caller inputs, not observed particle outputs.
Velocity overrides remain absent. All temporary logging was removed.

The same full-field replay compares 467,132 simulation fields and 9,373 ordered
particle draws over 300 ticks. HUD shake draws are classified as post-particle
external RNG inputs, with strict site order; the production M5 gate generates
those draws independently in `melee-if` at scheduler link 17.

## Yoshi's Story (Lane B2, 2026-09-09)

`idle_ys_spawns.json` contains the 26 common-bank puff requests logged from
production `melee-sim gate harness/scenarios/idle_ys_fox.toml`, which reported
600 ticks × 49 keys with zero divergences. The fixture uses the format above,
with `detach: true` recording `hsd_8039F6CC`'s pending attachment queue.
Each request contains only the production joint matrix (joint 201: map 2,
archive descendant 1), bank/kind/link and zero local position. No retail
particle outputs, future seeds or child-generator requests became inputs.
Temporary logging was removed after generation. To regenerate, log the Story
puff SpawnRequest in `frame.rs` and the following pending-generator enqueue.
The replay bakes/releases that attachment before particle execution, as in
retail. Its initial puff has no attachment and needs no fixture joint mapping.
The 600-tick replay compares 51,303 fields and 476 ordered draws, with no
mismatches and no display-cache exclusions.


## M5 A2 input fixtures

`jab_fd_fox_spawns.json`, `utilt_fd_marth_spawns.json` and
`shieldhit_fd_marth_spawns.json` were logged on 2026-09-09 from the production
Lane A2 gates; each reports 300 ticks, 49 keys, 0 divergences. They have
8/13, 12/16 and 10/22 external spawns/retained events respectively.
Use the method above, also recording the root-scale override and the
`clear: 0x600, set: 0x800` operations on normal hit sparks (2/306/307).
Flag operations apply to the immediately preceding spawn, even when several
generators attach to the same joint. The fixture helper asserts that joint.

When instrumenting `ParticleSystem::spawn`, exclude calls inside
`update_particle`: those are child-generator instructions that the replay must
execute itself. These fixtures contain no child requests or retail particle
outputs. They retain only external calls, used joint updates and expirations;
unchanged duplicate matrices are omitted. All temporary instrumentation and
its temporary serialization dependency were removed.

The replays compare 474,513 / 510,260 / 468,078 simulation fields and
9,642 / 9,900 / 9,441 ordered particle draws with zero mismatches. No new
AppSRT display-cache exclusions were added. `grab_fd_marth_startup_spawns.json` contains only the 9 external spawns / 10
events logged from production ticks 0–126 by the same method. Its enabled
startup replay compares 215,230 fields and 4,188 ordered draws, zero mismatches,
final seed `0x71afbd66` (1,960 existing display-cache exclusions). The full
300-tick grab replay is explicitly ignored: gameplay stops at tick 127's active
catch capsule, so linked capture/throw/missed-tech effects and their production
fixture are still missing. The startup helper checks the original 300-record
file lengths and compares 127 records; existing full replays are unchanged.

## Yoshi's Story match start (Lane B5, 2026-09-09)

`start_ys_spawns.json` was logged from production `melee-sim gate
harness/scenarios/start_ys_fox.toml` (600 x 49, zero divergences). It contains
38 external requests: 30 detached Randall puffs and eight fighter entry/landing
requests; 284 input events including joint updates/expiration. Temporary
instrumentation at `ParticleSystem::{spawn,update_joint,expire_joint}` and
`Simulation::tick` was removed. The 160 synchronous child requests (kinds
446/447, `ParticleSystem::drain_children`) were discarded: bytecode generates
those during replay. Unchanged matrices, unused joints, and map-2 updates
for already detached puffs were omitted; each puff carries its own current
production matrix. No retail particle outputs were used as fixture inputs.

`live_ys_start` uses `dust_replay.rs`, including the match-start tick-zero
scheduler boundary. It compares 215,202 fields and 4,864 ordered particle RNG
draws over 600 ticks, with zero mismatches and zero excluded fields.
