# Plan

Bottom-up along the call graph, gated on bit-exact traces against retail at
every step. `TRACKER.md` holds the current focus and backlog.

| # | Milestone | Gate | Status |
|---|---|---|---|
| 1 | Math and RNG (`gekko-math`) | Unit tests against compiled decomp C; hardware `frsqrte`/`fres` | Done; FMA sites audited against retail asm. |
| 2 | Archive, scheduler, animation, particles | Bone matrices and particle state vs Dolphin dumps | Done. |
| 3 | One fighter idle | `idle_fd_fox` 600 ticks bit-exact | Done. |
| 4 | Movement | Movement scenes per character | Done for Fox, Marth, Falco, Captain Falcon, Peach, Jigglypuff, Yoshi. |
| 5 | Combat, two fighters | Hits, shields, grabs, KOs | Done for Fox vs Marth on FD: two recorded human matches bit-exact; full interaction coverage including Sudden Death (`docs/MATCHUP_COMPLETENESS.md`). |
| 6 | Breadth | Every character and stage, idle to full matchup | In progress: see `TRACKER.md` "Status by area". |
| 7 | Slippi replay corpus | Zero divergence over thousands of replays | Parser and single-replay command exist; batch runner next. |
| 8 | Platform | Native app | First app done; pixel fidelity open. |

Menus, results screens, CPU AI and single-player modes stay out of scope
until milestone 7 is green.

## Choosing the next piece of work

Prefer what real games need and the gate cannot yet verify. Measure it with
Slippi divergences and explorer faults per hour of work. Shared-engine gaps
come first: they unblock every character and stage at once.
