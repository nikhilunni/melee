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
| 6 | Breadth | Every character and stage, idle to full matchup | In progress: ten supported Versus stages, with unwitnessed hazards guarded; 19 remain. All 26 characters are registered; Kirby has 24/25 copied neutrals (Sheik remains unported). Guarded FD/Battlefield CPU players, random containers, narrow Barrel owner transfer, ground Sword entries and empty Box/Barrel blasts are integrated. Moving Kongo barrel use, neutral Party Ball FALL/REST, log-standing, revival-floor entry, retained running-item program identity and pinned mouth-victim bottom cleanup are integrated. Each admission has complete recordings or explicit entry-only limits. Shared Stone contact handling, terminal HeavyGet entry, held Sword common F/B/Hi entry, bounded Kuriboh/ReDead births and coupled Klaptrap birth-tick path/item work are integrated; the paired Giant Punch invariant is source/data-proven without new behavior; unsupported mixed item-contact paths refuse explicitly, while Ness yo-yo instance binding awaits complete particle evidence (`TRACKER.md`). |
| 7 | Slippi replay corpus | Zero divergence over thousands of replays | In progress. `melee-sim replay-batch` runs a corpus with the Slippi codes each game was recorded with: 108 of 108 games complete on `public-v3.7` (964,782 matched frames), 486 of 502 on `public-v3.7-b` (5,102,652 matched frames). Five games stop; unsupported setups remain refused, including CPU levels absent from Slippi and stock-sharing teams. Lag inference and Slippi Online retain their documented comparison boundaries. Open: the remaining stops by kind (`TRACKER.md`), then a wider corpus. |
| 8 | Platform | Native app | In progress (`docs/APP.md`). A shared core (`melee-platform`) under a macOS host and a web host, with the disc image as the only input; the wave-22 checkpoint matches native/WASM over 6,003 hash lines and passes the release web build. Open: render-to-texture (FoD reflection, Stadium screen), gamepads and the other items in `TRACKER.md`. |

The six incoming handoffs and subsequent breadth slices are integrated.
Checkpoint source `f1288878` retains both replay corpora and has a fresh
3,490-scenario sweep with zero regressions and sixteen fixes against
`6d0c850e` (281 fixes against startup `0a99db31`). The combined release
workspace passes 2,006 tests; clippy, harness, schema, five WASI guard traps,
WASM hashes and web pass. `TRACKER.md` records current counts and source
provenance; performance retains unchanged budgets.

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

The next isolated candidate is common held Sword Down Air entry. Five
complete captures can potentially prove that terminal slice; next callbacks
remain guarded. A static character-table capability must close both normal
Link/YoungLink entry and their direct bounce restart, preserving midair
catches and actual-row/private distinctions. F/B/Hi entry and the coupled
Klaptrap birth are integrated. Klaptrap includes its stage-bank particle,
literal path/tangent math and narrow birth-frame DFS1 matrix proof; later
Ground/item lifecycle remains guarded. Ness's current yo-yo instance binding
lacks whole-capture particles and awaits add-only recording authorization.

Revival-floor entry is witnessed only on its 1,679-tick capture's final tick;
no later floor-exit observation is available. The second-live-log creation
is likewise on the final tick of its capture, without coexistence proof.
Party Ball's one 600-tick warm/cold capture admits only neutral left-facing
FALL/REST. Kongo barrel use has three complete captures and six warm/cold
scenarios; log-standing adds one whole warm/cold capture. Ground Sword work
admits actual common rows 57/63 and witnessed throws; common F/B/Hi
aerials admit only recorded held entry. Neutral/Down/private aerials and
Hi3/Lw4 remain guarded. Terminal monster, heavy and aerial data do not prove
later lifecycles. The CPU audit found no eligible additional branch in
existing complete configured-CPU recordings.
No new recordings are authorized in the current task.
