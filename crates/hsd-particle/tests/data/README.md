# External particle-input fixtures

Fixtures are tied to their recording generation. Regenerate them with the
commands below whenever the savestates/traces change. C13 regenerated the
supported Fox scenes against the 2026-09-09 replacement recordings. The replacement
recording boundary shifts later events by one tick (for example ledge 246→245 and 283→282). Each export first matches the
**new retail trace**, using exactly the ordinary gate importer, inputs,
scheduler and comparisons. No manual production instrumentation is needed.

The command writes only after the entire scenario gate succeeds. `--ticks N`
exports a prefix but still validates the entire scenario. A failed gate leaves
an existing output file untouched. Run from the repository root with the owned
local archives, savestates and traces installed.

## Regenerated Fox fixtures

```sh
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/shield_fd_fox.toml --out crates/hsd-particle/tests/data/shield_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/spotdodge_fd_fox.toml --out crates/hsd-particle/tests/data/spotdodge_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/roll_fd_fox.toml --out crates/hsd-particle/tests/data/roll_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/airdodge_fd_fox.toml --out crates/hsd-particle/tests/data/airdodge_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/wavedash_fd_fox.toml --out crates/hsd-particle/tests/data/wavedash_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/ledge_fd_fox.toml --out crates/hsd-particle/tests/data/ledge_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/jump_fd_fox.toml --out crates/hsd-particle/tests/data/jump_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/dash_fd_fox.toml --out crates/hsd-particle/tests/data/dash_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/start_fd_fox.toml --out crates/hsd-particle/tests/data/start_fd_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/start_bf_fox.toml --out crates/hsd-particle/tests/data/start_bf_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/start_dl_fox.toml --out crates/hsd-particle/tests/data/start_dl_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/jab_fd_fox.toml --out crates/hsd-particle/tests/data/jab_fd_fox_spawns.json
```

Jump, dash and FD start now use the common JSON reader. Their former Rust
schedules, alternate jump JSON, captured start-joint JSON and address-specific
Python extractor were removed. `live_fd` restores idle FD directly from its
initial capture and needs no external spawn fixture.

## Regenerate after C12

These files remain unchanged because the current importer rejects their saved
scheduler resume boundaries. These are the exact commands to run after C12:

```sh
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/idle_bf_fox.toml --out crates/hsd-particle/tests/data/idle_bf_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/idle_dl_fox.toml --out crates/hsd-particle/tests/data/idle_dl_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/idle_ys_fox.toml --out crates/hsd-particle/tests/data/idle_ys_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/start_ys_fox.toml --out crates/hsd-particle/tests/data/start_ys_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/jab_fd_marth.toml --out crates/hsd-particle/tests/data/jab_fd_marth_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/utilt_fd_marth.toml --out crates/hsd-particle/tests/data/utilt_fd_marth_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/shieldhit_fd_marth.toml --out crates/hsd-particle/tests/data/shieldhit_fd_marth_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/grab_fd_marth.toml --out crates/hsd-particle/tests/data/grab_fd_marth_startup_spawns.json --ticks 127
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/grab_fd_marth.toml --out crates/hsd-particle/tests/data/grab_fd_marth_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/tech_fd_marth.toml --out crates/hsd-particle/tests/data/tech_fd_marth_spawns.json
cargo run -q -p melee-sim -- fixture-spawns harness/scenarios/ko_fd_marth.toml --out crates/hsd-particle/tests/data/ko_fd_marth_spawns.json
```

All Marth/Falco/Falcon/Peach/Yoshi/Puff scenes also remain blocked on C12,
including those without a checked-in spawn fixture. Do not regenerate their
inputs from a partial run.

## Format and provenance

The JSON object maps completed tick ordinals to ordered external input events.
The permanent optional recorder labels events before scheduler execution and
records effect/stage requests immediately at the particle-system call site:

- Spawn: `bank`, `spawn`, `link`, `position`, optional `joint` (`id`, `matrix`),
  `transform` (translation, rotation, scale, status), `mirror`, and `detach`.
- Joint changes: `update` plus `matrix`, or `expire`.
- Caller flag operations: `flags_joint`, `clear`, and `set` on the just-spawned
  generator. Hit-spark root scaling follows the retail dispatch including kind 6.
- `external_randf` records the slash-orientation caller; `after_particles`
  places HUD requests after both particle passes.

Float inputs use `f32::to_bits()`. Retained calls keep their original order,
including multiple joint changes within a tick. Only externally referenced,
non-detached joint updates survive filtering; repeated unchanged matrices are
omitted. A new spawn matrix does not mark older attached generators updated.

The interpreter generates all particle-bytecode children itself. Fixtures
contain no child requests, particle outputs, expected generator/AppSRT state,
RNG results or injected seeds. The replay obtains initial state, retail RNG
calls and expected outputs independently from local captures. The command's
integration test checks parsed JSON equality against the regenerated ledge
fixture, prefix export, failure preservation, and zero absent-sink allocations.

Before C13, these inputs were produced by temporarily instrumenting production
calls, running a gate and removing the instrumentation. The permanent command
replaces that recipe for every file above. See
[the C13 report](../../../../docs/PORT_NOTES/C13_FIXTURES.md) for validation and
the recording-constant audit.
