# Plan

Bottom-up along the call graph, gated on bit-exact traces at every step.
`third_party/melee-decomp/tools/dep_graph.py` computes the ordering.

| # | Milestone | Gate | Status |
|---|---|---|---|
| 1 | Math and RNG (`gekko-math`) | Unit tests; goldens for `frsqrte` once tables are decided | RNG and FMA done. Estimates are placeholders. |
| 2 | Archive parsing and animation (`hsd-archive`, `hsd-anim`) | Load one character and one stage, match bone matrices of the wait animation | Header parser only |
| 3 | One fighter idle on Final Destination | `idle_fd_fox` 600 frames bit-exact | Scenario written; needs savestate and fighter-list walk |
| 4 | Scripted input: movement, jumps, ledges | Movement scenarios | |
| 5 | Two fighters: hitboxes, damage, knockback, shields | Combat scenarios | |
| 6 | Items, remaining stages, characters one by one | Per-character scenarios | |
| 7 | Slippi replay corpus | Zero divergence over thousands of replays | |

Menus, results screens, and single-player modes are out of scope until 7
is green.

## Immediate next steps

1. Transcribe `HSD_GObjEntities` and `HSD_GObj` layouts into
   `harness/schema/globals.yaml` and finish `fighter_bases()` in
   `harness/dolphin/trace_scenario.py`.
2. Decide the `frsqrte` table question (`crates/gekko-math/src/estimate.rs`).
   Preferred: extract the table empirically with a Dolphin scenario.
3. Port MSL `sinf`/`cosf`/`tanf` and their tables from `src/MSL/trigf.c`
   and `src/MSL/math_data.c`.
4. Record the `idle_fd_fox` savestate and produce the first real
   `expected.jsonl`.
