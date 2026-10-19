# The oracle

Melee is a deterministic function of (initial state, RNG seed, inputs). The
retail game running in Dolphin is treated as ground truth, and the Rust port
is checked against it to the bit.

## Pipeline

```
scenario.toml ──> Dolphin + trace_scenario.py ──> raw.jsonl ──> decode.py ──> expected.jsonl
                                                                                  │
scenario.toml ──> melee-sim ────────────────────────────────────────────> actual.jsonl
                                                                                  │
                                                                     melee-diff ──┴──> first divergence
```

- **Raw dump.** The Dolphin script reads the RNG seed and the raw bytes of
  every fighter struct each frame. It never modifies the game.
- **Schema.** `harness/schema/*.yaml` maps struct offsets to field names,
  copied from the decomp headers. Both decoders are driven by it.
- **Canonical trace.** JSON Lines of `{frame, phase, state}`. Floats carry
  their bit pattern. One ULP is a divergence.
- **Phases.** `frame_end` first. Later, code breakpoints at GObj process
  boundaries (input, fighter update, collision, items, camera) add
  intra-frame phases so a divergence is localised to a subsystem on the
  first run.

## External events

Pads are not the only input. A few things the game logic reads come from the
platform and no game state predicts them; they are inputs, recorded like
pads, never compared as state. Today there is one: the completion of Pokémon
Stadium's asynchronous form-archive read, which `grStadium_801D42B8` polls
once per tick (its latency is the disc's, in Dolphin the emulated DVD
timing, and varies between recordings of the same match).

- **Retail.** On Pokémon Stadium the tick tracer adds `stage_io` to each
  record (map 2's read-pending bit and controller phase at the tick's end);
  `decode.py` turns it into `events.stage_read_completed` per tick: true on
  the tick whose poll found the read complete (the phase leaves 1).
  Other stages' traces carry neither field.
- **Port.** `melee_lib::ExternalEvents` is passed per tick beside the pads
  (`Match::step_with_events`; `Match::step` uses the defaults), and
  `Match::consumed_events` reports each poll's outcome. `StageRead::Default`
  is the port's documented latency policy (a fixed poll per form archive,
  measured from retail's first reads); `Completed`/`InFlight` replay a
  recording; `Unrecorded` fails the tick if the stage polls.
- **Oracle.** `melee-sim gate`/`triage`/`particles-diff` replay each tick's
  recorded event from the expected trace, as they replay `inputs`; a trace
  without `events` is `Unrecorded`, so a stage that polls fails closed
  instead of guessing. Dry runs and Slippi replays use the defaults.
- **Recordings.** `melee-replay` stores the polls the port consumed
  (`stage_reads`, optional), so a replay of an explorer or app recording
  reproduces it exactly even if the default policy changes.

## Particles in the gate

`record.py` writes a particle dump beside the tick trace
(`<name>.particles.jsonl`): the generator list and the sixteen particle
lists in order with every field, the pending list, the counts, the family
counter and the seed, at each tick's end. `melee-sim gate`, `sweep`, `triage` and `m5_gate`
compare the port's particle system with it on every tick and report a
difference like any other key (`phase particles`, the key, both values, and
the object's bank, program and family id; `triage` adds both sides' lists).
The check is on by default since 2026-10-19, when the last particle fault was
fixed (user decision); `gate --no-particles` compares fighters, items and the
seed only. A scenario without a dump, or whose dump is not the tick trace's
game, reports why it was not compared and still passes. The code
is `crates/melee-sim/src/particle_gate.rs`.

- **What is compared.** Every key of the dump except the AppSRT display
  caches (`particles.appsrt[i]`, which need the camera and the render
  schedule). The port's snapshot must hold exactly the dump's keys.
- **Cost.** A dump is about 100 kB of JSON per tick, so the gate never
  holds one. It is read once, as a stream, into
  `<name>.particles.digest` beside it: per record two 64-bit digests, the
  seed and the dump pass's display clock. The sidecar is derived data; its
  header names the dump files it was read from (length, modification time)
  and it is rebuilt when they change. Per tick the gate digests the port's
  particle system through the snapshot's field sink
  (`Simulation::write_particle_fields`, no keys spelled) and compares one
  number; only at a difference does it read the dump's record of that tick.
  Nothing runs in the simulation's tick.
- **A dump is its own Dolphin run.** It is compared only if it is the game
  the tick trace recorded, by `record.py`'s rule: equal end-of-tick seeds on
  every record, and every record the pass was asked for (a pass capped with
  `--particle-ticks` is whole). Otherwise the gate passes as before and
  prints why the dump was not compared (`particles: dump not compared: ...`).
- **Display cadence.** particleSort runs in the display pass, so the order
  of a particle list follows the cadence of the run, and each Dolphin run
  has its own. Where the dump's display clock (`ps_frame`) puts passes on
  other ticks than the tick trace's, the check steps a second simulation
  under the dump's clock beside the gate's and compares that one.
- **Dumps without a display clock** (tracers before 2026-09-26) have an
  unknown cadence; `vi_frame` does not give it (a pass can fall between two
  ticks of one VI frame). Their lists are compared as bags: every
  particle's fields, but not its place in its list. Generators, counts and
  the seed are compared as always.
- **No dump**: the scenario gates as before.

## Golden function tests

For the last mile, break on entry and exit of a target function in Dolphin,
capture arguments and touched memory, and emit fixtures under
`harness/goldens/`. Each ported Rust function gets real-game unit tests
before it is integrated.

## Scenario corpus

1. Hand-authored scenarios under `harness/scenarios/`, one per milestone.
2. Slippi replays. They record every player's inputs and the RNG seed per
   frame, which makes them millions of free acceptance tests once the core
   is complete.
