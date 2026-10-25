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
| 6 | Breadth | Every character and stage, idle to full matchup | In progress: ten supported Versus stages, with unwitnessed hazards guarded; 19 remain. All 26 characters are registered; Kirby has 24/25 copied neutrals (Sheik remains unported). Guarded FD/Battlefield CPU players, random containers, narrow Barrel owner transfer, ground Sword entries and empty Box/Barrel blasts are integrated. Their witness boundaries remain explicit; next are Party Ball and moving Kongo barrel work (`TRACKER.md`). |
| 7 | Slippi replay corpus | Zero divergence over thousands of replays | In progress. `melee-sim replay-batch` runs a corpus with the Slippi codes each game was recorded with: 108 of 108 games complete on `public-v3.7` (964,782 matched frames), 486 of 502 on `public-v3.7-b` (5,102,652 matched frames). Five games stop; unsupported setups remain refused, including CPU levels absent from Slippi and stock-sharing teams. Lag inference and Slippi Online retain their documented comparison boundaries. Open: the remaining stops by kind (`TRACKER.md`), then a wider corpus. |
| 8 | Platform | Native app | In progress (`docs/APP.md`). A shared core (`melee-platform`) under a macOS host and a web host, with the disc image as the only input; the wave-15 checkpoint matches native/WASM over 6,003 hash lines and passes the release web build. Open: render-to-texture (FoD reflection, Stadium screen), gamepads and the other items in `TRACKER.md`. |

The six incoming handoffs and three wave-15 breadth slices are integrated.
Production source `af5f2587` retains both replay corpora and has a combined
3,485-scenario sweep with zero regressions and 35 fixes against `bf98d61d`.
The full post-wave-15 release workspace, clippy, harness, schema, WASM and
web checkpoint passes. Performance timing and shared-code duplication pass;
existing size and Kirby-label budget failures remain. `TRACKER.md` records
current counts, source provenance and the unchanged budgets.

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

The next concrete implementation uses existing recordings only: Party Ball
FALL/REST from its left-facing 600-tick capture, then the Kongo warm saved
AObj/controller-clock correction and complete held/launch witnesses. Ground
Sword work admitted only actual common rows 57/63 and witnessed throws;
private/aerial/Hi3/Lw4 rows stay guarded. Barrel primary-owner transfer covers
an unowned REST barrel at contact, while empty-blast recordings cover the
whole observed lifecycle. Birth/entry-only monster, heavy and aerial data
do not authorize later callbacks. No new recordings are authorized in the
current task.
