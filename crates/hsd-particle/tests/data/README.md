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
