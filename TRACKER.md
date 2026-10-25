# Tracker

Legend: `[ ]` todo, `[~]` in progress, `[x]` done, `[!]` blocked.
Keep current facts and next work here; earlier checkpoints live in git history
and `docs/PORT_NOTES/`.

## Current state (2026-10-24, wave 15 integrated; functional checkpoint complete)

- **Three breadth slices are integrated:** narrow primary-article owner
  transfer to an unowned REST Barrel, held Sword ground inputs/entries,
  and whole Box/Barrel empty blasts with deferred callback phase retained.
  Production source is `af5f2587`; later commits through `58118743` add
  regression coverage. Scope and evidence: `docs/PORT_NOTES/WAVE15_BREADTH.md`.
  The six earlier handoffs are historical (`WAVE14_INTEGRATION.md`).
- **Combined sweep:** 3,485 scenarios, **3,290 particle-exact**, 115 not
  comparable, 79 existing local failures and one existing particle fault.
  **Zero regressions, 35 fixes** against `bf98d61d`; **228 fixes** against
  startup `0a99db31`. Incomplete recordings and local failures are not
  accepted divergences in registered witnesses.
- **Replay corpora remain unchanged:** `public-v3.7` completes **108/108**
  (964,782 matched frames); `public-v3.7-b` completes **486/502**
  (5,102,652 frames), retaining every path's category and matched count.
  CPU replay setup, stock sharing and other unsupported capabilities stay
  refused. Slippi does not record the CPU levels needed for admission.
- **Parity inventory** (`target/parity/wave15-integrated/`,
  `docs/PARITY.md`): all retail **29.2% ported/partial, 37.9% with body/row**;
  historical in-game subtotal **44.9% / 56.9%**, unchanged when rounded.
  Of 19,827 functions: 4,202 ported, 201 partial, 1,361 body, 1,378 row,
  221 referenced and 12,464 not started. **252 fail-closed sites**:
  233 unimplemented and 19 bail sites. Citation shifts include changed
  comments and guard attribution; these code-byte estimates are not a retail-proof score.
- **Combined release checkpoint passes:** 1,940 workspace tests, zero
  failures, one existing ignore; m4 345, m5 297 and harness 407. Workspace
  clippy is clean. Schema checks report zero layout violations; WASM pure
  suites, four guarded spline subprocess checks, 6,003 identical native/WASM
  hash lines and the release web build pass. Expanded actual API allocation
  and capture checks pass; the 37-case strict random-item group includes
  the two combined Sword/container witnesses. Release only; no debug run.
- **Performance gate: existing budget failures remain.** At `58118743`,
  time passes (18.368 ms per 600 ticks; 55.729 ms load). Stripped size is
  7,605,840 bytes and text size 6,848,512 bytes, above unchanged ceilings;
  Kirby's one duplicate label still exceeds baseline zero. Shared-code
  duplication passes: 20 labels in melee-ft, 94 across crates. The report
  is appended to `docs/PERF.md`; no threshold or baseline was increased.

## Next (breadth before individual replay fixes)

1. [x] Close the wave 15 functional checkpoint and quiet performance
   measurement. Full-sweep/replay evidence and provenance are under
   `target/sweeps/58118743*`; completed worktrees have been removed.
   Performance budgets still require the user decision above.
2. [~] **Party Ball** (wave 16, 2026-10-24): bounded
   FALL/REST from the existing left-facing
   600-tick capture, following the integrated Resume phase correction.
   Opening, contents and other unproved continuations remain guarded.
3. [~] **Kongo moving barrel** (wave 16, 2026-10-24): restore
   the saved live AObj clock
   independently of the controller's future rate, then prove actual
   held-position and launch behavior against complete existing captures.
   Keep entry-only, second-live-log and other hazard limits explicit.
   Plan: `target/wave14/kongo-search/moving-barrel-implementation-plan.md`.
4. [ ] Continue heavy pickup/carry/throws, other item-use rows and random
   kinds, **19 remaining Versus stages**, copied Sheik Needle Storm, wider
   CPU stage awareness, modes, retail menus/results/single-player and pause.
   Monster births, heavy entries and held aerial entries do not prove
   later callbacks; keep them guarded. Nothing is out of scope.
   **No new recordings are authorized in the current task**; use existing
   protected data in place.
5. [ ] Replay depth after breadth: five existing stops remain in the second
   corpus (Kirby/Game & Watch aerial Fire near FoD's edge; two proven
   unconsumed controller polls; two one-ulp Nana positions). The other
   unfinished games are refused capabilities. Preserve the distinction
   between a replay stop, an exact retail witness and an unsupported setup
   (`docs/SLIPPI.md`).

## Status by area

**Characters:** all 26 are registered. Movement, combat and character
specials have recorded witness families. Kirby has **24/25 copied neutral
specials**; copied Sheik Needle Storm remains unported despite an existing
start boundary (`docs/PORT_NOTES/KIRBY.md`). Inhale/item and hand-pose gaps
remain. Character differences use static tables and family crates.

**Stages:** ten supported Versus stages: the six tournament stages, Yoshi's
Island 64, Kongo Jungle 64, Kongo Jungle and Jungle Japes. Supported does
not mean every hazard is enabled: Kongo barrels/log/event and other
unwitnessed branches retain explicit guards. Nineteen stages remain.
Stadium form-archive read completion is a recorded external input; ordinary
runs use documented default latency (`docs/APP.md`).

**Items:** shared engine and character articles, Peach's rare pulls and
Sword swings are ported. Random spawning and Capsule/Box/Barrel, including
empty blasts, are witnessed for two-player FD/Battlefield setups. Ground
Sword admissions add only common AttackLw3 (57), AttackHi4 (63) and their
witnessed throw routing; private, aerial, Hi3 and Lw4 rows stay guarded.
Barrel ownership adds only a primary article owner to an unowned REST
barrel; other ownership contexts remain guarded. Party Ball, other random
kinds and heavy fighter rows remain backlog. Entry-only recordings do not
prove later callbacks.

**CPU:** Nana follow/recovery behavior remains. Versus CPU players are now
admitted in the witnessed FD/Battlefield envelope, with character-owned
static profiles and 95 registered witnesses. Recovery, command and item
branches outside that envelope fail closed. A partially created primary
fighter cannot be warm-imported; its cold scenario remains supported.
CPU replay admission is still refused because Slippi lacks CPU levels.

**Match flow:** FFA and witnessed teams/doubles are integrated. Stock
sharing and timed-team ranking remain guarded. Keep successful team
completion proof distinct from refusing an unsupported stock-return setup.

**App:** native macOS and web hosts share `melee-platform`; the disc is the
only game-data input. The app's own menus do not count as retail menu
ports. Remaining renderer/gamepad/menu work is in `docs/APP.md` and
`docs/DESIGN.md`. Historical memory/clone measurements and earlier
checkpoints are preserved in `ee769ee9:TRACKER.md` and `docs/PERF.md`.

## Verification boundaries and backlog

- Particle comparison is on by default in gate/sweep/m5. Incomplete dumps
  remain explicitly not comparable; a partial ledger test states and
  checks its exact prefix. Never loosen retail expectations to pass.
- Replay comparisons preserve setup/code choices and lag-inference
  boundaries. Slippi Online has documented comparison differences;
  inferred lag frames are counted separately (`docs/SLIPPI.md`).
- Every-tick recordings are required to diagnose display-pass behavior.
  Older two-ticks-per-render particle/bone/camera dumps do not prove it.
  No existing protected data is replaced or moved in this task.
- `Match` allocation checks cover step/observe/capture, including CPU and
  container behavior. Clone below 1 ms remains a separate backlog item.
- User-owned open decisions: performance budgets; old dump re-recording;
  Slippi Dolphin unfused math policy; default lag-frame inference; pushing
  main/first CI run. No push is authorized.
- Fresh retail branch coverage is separate from the citation inventory;
  see `docs/PORT_NOTES/RETAIL_COVERAGE.md`. Before assigning historical
  fault packets, verify current code, guards and available witnesses.

## Decisions still in force

| Date | Decision |
|---|---|
| 2026-09-08 | Layered workspace: one crate per subsystem and per character (build time). |
| 2026-09-08 | Internal state need not match retail layout; `Snapshot` plus `harness/schema/` handle comparison. Traces compare floats by bit pattern. |
| 2026-09-08 | The oracle reads retail memory through Felk's Dolphin scripting fork; never a modified DOL. |
| 2026-09-08 | Slippi fixtures come from hohav/peppi (MIT), not slippi-js (LGPL). |
| 2026-09-08 | The empirically captured `frsqrte`/`fres` tables are committed (user decision). |
| 2026-09-08 | Melee-specific on-disc structs (`coll_data`, `ftData`, `map_head`) are read by the owning game crate in a `desc` module; `hsd-archive` stays HSD-only. |
| 2026-09-09 | Scripted scenarios drive Dolphin with a TOML input schedule; the port replays the pads recorded at each tick boundary (`melee-sim/src/inputs.rs`). |
| 2026-09-09 | Motion-state tables, static character tables, family crates, concrete core and shell, no per-tick allocation (architecture rules in `AGENTS.md`). |
| 2026-09-26 | Record with headless Dolphin by default; windowed only for human play and menu driving. |
| 2026-09-26 | The explorer corpus starts from registered retail boundaries, so every case is replayable in Dolphin. |
| 2026-10-24 | Subagents on any model, run widely in parallel (supersedes the 2026-09-28 Opus-only rule). `AGENTS.md` is the single instructions file for every agent. |
| 2026-09-28 | Perf size baseline raised to 6,444,304 stripped bytes after wave C (user). |
| 2026-09-28 | Slippi replays run with the Slippi codes they were recorded with, ported from slippi-ssbm-asm and cited; codes a replay does not name are inferred only between discrete known versions, and frame zero checks the choice. |
| 2026-10-04 | Perf size floor raised to 6,549,232 stripped bytes for five new characters; each new character crate accepted at one duplicate label (the shared `CharacterTable` closure), as every older one is (user). |
| 2026-10-03 | Where a recording's date allows UCF 0.73 or 0.74, the runner runs both from the recorded inputs and the recorded facing at the first dashback they disagree on picks the version, which then runs the whole comparison from frame one (never a switch mid-run); the report prints it and `--controller-fix` overrides it (user, 2026-10-03: coordinator's call). |
| 2026-09-28 | Asynchronous disc reads that affect gameplay (Pokémon Stadium's forms) are external inputs, recorded from retail and replayed like pads; standalone runs use a documented default. |
