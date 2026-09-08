# Plan

Bottom-up along the call graph, gated on bit-exact traces at every step.
`third_party/melee-decomp/tools/dep_graph.py` computes the ordering.

| # | Milestone | Gate | Status |
|---|---|---|---|
| 1 | Math and RNG (`gekko-math`) | Unit tests; goldens for `frsqrte` once tables are decided | RNG, FMA, and all in-tree MSL routines done and verified against natively compiled decomp C. `frsqrte`/`fres` are placeholders; 25 fusion sites await retail asm. |
| 2 | Archive parsing and animation (`hsd-archive`, `hsd-anim`) | Load one character and one stage, match bone matrices of the wait animation | Archive parsing complete with synthetic tests. Animation not started. |
| 3 | One fighter idle on Final Destination | `idle_fd_fox` 600 frames bit-exact | Scenario written; fighter-list walk done and tested. Needs disc, Dolphin build, savestate. |
| 4 | Scripted input: movement, jumps, ledges | Movement scenarios | |
| 5 | Two fighters: hitboxes, damage, knockback, shields | Combat scenarios | |
| 6 | Items, remaining stages, characters one by one | Per-character scenarios | |
| 7 | Slippi replay corpus | Zero divergence over thousands of replays | `slp` crate parses replays to scenarios and expected traces. |

Menus, results screens, and single-player modes are out of scope until 7
is green.

## Immediate next steps

1. Build the Dolphin scripting fork for macOS arm64 (`docs/DOLPHIN.md`).
2. Decide the `frsqrte` table question (`crates/gekko-math/src/estimate.rs`).
   Preferred: extract the table empirically with a Dolphin scenario.
3. Port `hsd-gobj` (scheduler) and start `hsd-anim` (JObj/AObj/FObj).
   Port `lbtrigf.c` (atan2f and friends) into `melee-lb`.
4. Record the `idle_fd_fox` savestate and produce the first real
   `expected.jsonl`.
