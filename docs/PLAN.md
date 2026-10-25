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
| 6 | Breadth | Every character and stage, idle to full matchup | In progress: ten supported Versus stages, with unwitnessed hazards guarded; 19 remain. All 26 characters are registered; Kirby has 24/25 copied neutrals (Sheik remains unported). Guarded FD/Battlefield CPU players, random containers, narrow Barrel owner transfer, ground Sword entries and empty Box/Barrel blasts are integrated. Moving Kongo barrel catches/holds/launches and neutral Party Ball FALL/REST are now integrated; next are log-standing and revival-floor entry (`TRACKER.md`). |
| 7 | Slippi replay corpus | Zero divergence over thousands of replays | In progress. `melee-sim replay-batch` runs a corpus with the Slippi codes each game was recorded with: 108 of 108 games complete on `public-v3.7` (964,782 matched frames), 486 of 502 on `public-v3.7-b` (5,102,652 matched frames). Five games stop; unsupported setups remain refused, including CPU levels absent from Slippi and stock-sharing teams. Lag inference and Slippi Online retain their documented comparison boundaries. Open: the remaining stops by kind (`TRACKER.md`), then a wider corpus. |
| 8 | Platform | Native app | In progress (`docs/APP.md`). A shared core (`melee-platform`) under a macOS host and a web host, with the disc image as the only input; the wave-15 checkpoint matches native/WASM over 6,003 hash lines and passes the release web build. Open: render-to-texture (FoD reflection, Stadium screen), gamepads and the other items in `TRACKER.md`. |

The six incoming handoffs and subsequent breadth slices are integrated.
Production source `93683489` retains both replay corpora and has a combined
3,488-scenario sweep with zero regressions and 14 fixes against `486aaba1`.
The full post-wave-16 release workspace, clippy, harness, schema, WASM and
web checkpoint passes. `TRACKER.md` records current counts, source provenance
and the unchanged performance budgets.

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

The next concrete implementations use existing recordings only: Kongo log
standing through a complete 2,829-tick capture, and the revival-floor callback's
actual final-tick Wait/protection/color entry. The latter 1,679-tick capture
has no later floor-exit observation; existing downstream callbacks are reused.
Party Ball's one 600-tick warm/cold capture admits only neutral left-facing
FALL/REST. Kongo barrel use has three complete captures and six warm/cold
scenarios. Ground Sword work admits actual common rows 57/63 and witnessed
throws; private/aerial/Hi3/Lw4 rows stay guarded. Entry-only monster, heavy,
aerial and second-log data do not prove later lifecycles. The CPU audit found
no eligible additional branch in existing complete configured-CPU recordings.
No new recordings are authorized in the current task.
