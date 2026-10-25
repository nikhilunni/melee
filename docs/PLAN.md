# Plan

Bottom-up along the call graph, gated on bit-exact traces against retail at
every step. `TRACKER.md` holds the current focus and backlog.

| # | Milestone | Gate | Status |
|---|---|---|---|
| 1 | Math and RNG (`gekko-math`) | Unit tests against compiled decomp C; hardware `frsqrte`/`fres` | Done; FMA sites audited against retail asm. |
| 2 | Archive, scheduler, animation, particles | Bone matrices and particle state vs Dolphin dumps | Done. |
| 3 | One fighter idle | `idle_fd_fox` 600 ticks bit-exact | Done. |
| 4 | Movement | Movement scenes per character | Done for all 26 characters. |
| 5 | Combat, two fighters | Hits, shields, grabs, KOs | Done for Fox vs Marth on FD: two recorded human matches bit-exact; full interaction coverage including Sudden Death (`docs/MATCHUP_COMPLETENESS.md`). Every registered character's combat is exact on its explorer batches vs Fox and on ten cross-matchups. |
| 6 | Breadth | Every character and stage, idle to full matchup | Stages done: the six tournament stages (FD, BF, DL, YS, FoD, PS) run full explorer matches exactly. Characters: all 26 registered with their specials; Kirby's copy abilities are not ported (open items in `TRACKER.md`). |
| 7 | Slippi replay corpus | Zero divergence over thousands of replays | In progress. `melee-sim replay-batch` runs a corpus with the Slippi codes each game was recorded with: 108 of 108 games complete on `public-v3.7`, 484 of 502 on `public-v3.7-b` (13 refused at setup: doubles, CPU players, other stages), twelve of them with an inferred console lag frame and fifteen of them Slippi Online games. Open: the remaining stops by kind (`TRACKER.md`), then a wider corpus. |
| 8 | Platform | Native app | In progress (`docs/APP.md`). A shared core (`melee-platform`) under a macOS host and a web host, with the disc image as the only input; all 25 characters and six stages render, and the wasm32 sim is bit-identical to native. Open: render-to-texture (FoD reflection, Stadium screen), gamepads and the other items in `TRACKER.md`. |

The objective is 100% retail parity, with no scope exclusions (user,
2026-10-24). Retail menus, results, CPU players and single-player modes are
part of the backlog. The app's current menus are its own; they do not count
as ports of retail menus.

## Choosing the next piece of work

Work breadth before replay depth: close fail-closed sites and port unstarted
retail code, with recordings proving each enabled branch. Rank bounded tasks
by retail code and fail-closed branches they can close. Shared-engine gaps
come first when they unblock several characters, items or stages. Use the
function inventory and explorer faults to choose work; chase individual
replay divergences after the breadth backlog.
