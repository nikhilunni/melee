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
