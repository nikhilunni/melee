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
