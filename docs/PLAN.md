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
| 6 | Breadth | Every character and stage, idle to full matchup | In progress: ten supported Versus stages, with unwitnessed hazards guarded; 19 remain. All 26 characters are registered; Kirby has 24/25 copied neutrals (Sheik remains unported). Guarded FD/Battlefield CPU players, random containers, narrow Barrel owner transfer, ground Sword entries and empty Box/Barrel blasts are integrated. Moving Kongo barrel use, neutral Party Ball FALL/REST, log-standing, revival-floor entry, retained running-item program identity and pinned mouth-victim bottom cleanup are integrated. Each admission has complete recordings or explicit entry-only limits. Shared Stone contact handling, terminal HeavyGet entry, held Sword common F/B/Hi/Lw entry, bounded Kuriboh/ReDead births and coupled Klaptrap birth-tick path/item work are integrated; shared teardown-before-sealed-effect ordering is witnessed through a complete 700-tick landing; the paired Giant Punch invariant is source/data-proven without new behavior; unsupported mixed item-contact paths refuse explicitly, while Ness yo-yo instance binding awaits complete particle evidence (`TRACKER.md`). |
| 7 | Slippi replay corpus | Zero divergence over thousands of replays | In progress. `melee-sim replay-batch` runs a corpus with the Slippi codes each game was recorded with: 108 of 108 games complete on `public-v3.7` (964,782 matched frames), 486 of 502 on `public-v3.7-b` (5,102,652 matched frames). Five games stop; unsupported setups remain refused, including CPU levels absent from Slippi and stock-sharing teams. Lag inference and Slippi Online retain their documented comparison boundaries. Open: the remaining stops by kind (`TRACKER.md`), then a wider corpus. |
| 8 | Platform | Native app | In progress (`docs/APP.md`). A shared core (`melee-platform`) under a macOS host and a web host, with the disc image as the only input; the wave-24 checkpoint matches native/WASM over 6,003 hash lines and passes the release web build. Open: render-to-texture (FoD reflection, Stadium screen), gamepads and the other items in `TRACKER.md`. |

The six incoming handoffs and subsequent breadth slices are integrated.
Checkpoint source `f570da10` retains both replay corpora and has a fresh
3,490-scenario sweep with zero changed rows against wave 23 (`55fe9480`)
(287 fixes against startup `0a99db31`): 3,354 particle-exact, 115 not comparable,
21 existing failures and zero particle faults. The combined release workspace
passes 2,020 tests; clippy, harness, schema, five WASI guard traps, 6,003 WASM
hash lines and web pass. Two command-owner guards are now source/data-proven
invariants, without admitting gameplay; 297 fail-closed sites remain. The
function inventory's covered-byte percentages and performance budgets are
unchanged. See `docs/PORT_NOTES/WAVE24_BREADTH.md`.

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

Common held Sword F/B/Hi/Lw terminal entries are integrated, with a static
character-table capability preserving Link/YoungLink's normal entry and direct
bounce restart refusal. Later common callbacks stay guarded. The shared
teardown-before-sealed-effects correction fixes the complete 700-tick Yoshi's
Story particle/RNG witness. Bounded Klaptrap/monster births remain integrated;
Klaptrap's matrix proof is only its recorded birth-frame DFS1 SRT/world matrix,
not a full lifecycle or pixel proof. Peach's private float item policy is
pre-existing and outside these new common-row witnesses.

Two original command-owner invariants (Swing42/Smash42) are integrated.
A bounded search found no eligible third invariant, without claiming
exhaustive impossibility. Ness's current yo-yo binding needs a complete fresh
4,775-tick capture; its old particle/meta stream ends at 3,340. The user
explicitly authorized new captures on 2026-10-25, superseding the prior
recording restriction. Ness is first in the quiet queue, followed by later
heavy carry/throw, Sword/monster callbacks, copied Sheik and another stage.
All remain in scope.

Revival-floor entry is witnessed only on its 1,679-tick capture's final tick;
no later floor-exit observation is available. The second-live-log creation
is likewise on the final tick of its capture, without coexistence proof.
Party Ball's one 600-tick warm/cold capture admits only neutral left-facing
FALL/REST. Kongo barrel use has three complete captures and six warm/cold
scenarios; log-standing adds one whole warm/cold capture. Ground Sword work
admits actual common rows 57/63 and witnessed throws; common F/B/Hi/Lw
aerials admit only recorded held entry. Neutral, private Game & Watch aerials
and Hi3/Lw4 remain guarded; Peach float uses its separate pre-existing policy.
Terminal monster, heavy and aerial data do not prove later lifecycles. The CPU audit found no eligible additional branch in
existing complete configured-CPU recordings.
New captures and required boundaries are authorized. Use fresh names, retain
existing data, and record at modest concurrency without competing heavy checks.
