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
