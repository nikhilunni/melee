# Plan

Bottom-up along the call graph, gated on bit-exact traces against retail at
every step. `TRACKER.md` holds the current focus and backlog.

| # | Milestone | Gate | Status |
|---|---|---|---|
| 1 | Math and RNG (`gekko-math`) | Unit tests against compiled decomp C; hardware `frsqrte`/`fres` | Done; FMA sites audited against retail asm. |
| 2 | Archive, scheduler, animation, particles | Bone matrices and particle state vs Dolphin dumps | Done. |
| 3 | One fighter idle | `idle_fd_fox` 600 ticks bit-exact | Done. |
| 4 | Movement | Movement scenes per character | Done for every registered character (20 of 26). |
| 5 | Combat, two fighters | Hits, shields, grabs, KOs | Done for Fox vs Marth on FD: two recorded human matches bit-exact; full interaction coverage including Sudden Death (`docs/MATCHUP_COMPLETENESS.md`). Every registered character's combat is exact on its explorer batches vs Fox and on ten cross-matchups. |
| 6 | Breadth | Every character and stage, idle to full matchup | Stages done: the six tournament stages (FD, BF, DL, YS, FoD, PS) run full explorer matches exactly. Characters: 20 of 26 registered with their specials (open items and the six unregistered kinds in `TRACKER.md`). |
| 7 | Slippi replay corpus | Zero divergence over thousands of replays | Next. Parser and single-replay command exist; needs a replay corpus, a batch runner and Slippi Online's initialization/seed resets. |
| 8 | Platform | Native app | First app done; pixel fidelity open. |

Menus, results screens, CPU AI and single-player modes stay out of scope
until milestone 7 is green.

## Choosing the next piece of work

Prefer what real games need and the gate cannot yet verify. Measure it with
Slippi divergences and explorer faults per hour of work. Shared-engine gaps
come first: they unblock every character and stage at once.
