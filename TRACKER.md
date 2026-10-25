# Tracker

Legend: `[ ]` todo, `[~]` in progress, `[x]` done, `[!]` blocked.
Keep current facts and next work here; earlier checkpoints live in git history
and `docs/PORT_NOTES/`.

## Current state (2026-10-24, wave 16 integrated; functional checkpoint complete)

- **Two more breadth slices are integrated:** Kongo's saved live barrel
  animation clock and actual catches/holds/launches (`cefbc068`), and neutral
  left-facing Party Ball FALL/REST (`93683489`). Source is `93683489`;
  evidence and limits: `docs/PORT_NOTES/WAVE16_BREADTH.md` and
  `PARTY_BALL_FALL_REST.md`. The earlier handoffs and wave 15 slices remain
  integrated; their reports preserve historical measurements.
- **Combined sweep:** 3,488 scenarios, **3,307 particle-exact**, 115 not
  comparable, 65 unchanged local failures and one unchanged particle fault.
  **Zero regressions, 14 fixes** against `486aaba1`; **242 fixes** against
  startup `0a99db31`. Three new cold scenarios share their warm captures.
  Incomplete recordings and local failures are not accepted divergences.
- **Replay corpora remain unchanged:** `public-v3.7` completes **108/108**
  (964,782 matched frames); `public-v3.7-b` completes **486/502**
  (5,102,652 frames), retaining every path's category and matched count.
  CPU replay setup and stock sharing remain refused capabilities.
- **Parity inventory** (`target/parity/wave16-integrated/`,
  `docs/PARITY.md`): all retail **29.3% ported/partial, 38.0% with body/row**;
  historical in-game subtotal **44.9% / 57.0%**. Of 19,827 functions:
  4,208 ported, 201 partial, 1,362 body, 1,381 row, 229 referenced and
  12,446 not started. **269 fail-closed sites**: 250 unimplemented, 19 bail.
  New explicit Party Ball guards increase the count while its witnessed
  path becomes exact. Citations and retail bytes are a proxy for progress.
- **Combined release checkpoint passes:** 1,947 workspace tests, zero
  failures, one existing ignore; m4 345 and m5 299. Workspace all-target
  clippy is clean; harness 407 and schema zero layout violations. WASM pure
  suites, four expected spline traps, 6,003 identical native/WASM hash lines
  and the release web check pass. Actual API allocation/capture checks pass.
  Release only; no debug run.
- **Performance gate retains existing budget failures:** at `93683489`,
  time passes (18.775 ms per 600 ticks; 55.560 ms load). Stripped size is
  7,639,936 bytes and text size 6,881,280 bytes, above unchanged ceilings;
  Kirby's one duplicate label still exceeds baseline zero. Shared-code
  duplication passes: 20 melee-ft labels, 94 across crates. The complete
  report is appended to `docs/PERF.md`; no budget was increased.

## Next (breadth before individual replay fixes)

1. [x] Close wave 16 documentation and quiet performance measurement.
   Verified sweep/replay artifacts are under `target/sweeps/93683489*`;
   merged worktrees are removed after preserving their acceptance evidence.
2. [ ] **Kongo log-standing:** wire the existing `Log::stood_on` callback,
   preserve Sink/Remove guards, and prove the complete 2,829-tick capture
   plus cold initialization. Plan: `target/wave16/kongo-log-search/plan.md`.
3. [ ] **Revival-floor entry:** copy resolved position, grant protection
   and color, then reuse normal landing/Wait entry. The 1,679-tick recording
   ends on the actual transition; it provides no later floor-exit ticks.
   Plan: `target/wave16/revival-floor-search/plan.md`.
4. [ ] Continue heavy pickup/carry/throws, other item-use rows and random
   kinds, **19 remaining Versus stages**, copied Sheik Needle Storm, wider
   CPU stage awareness, modes, retail menus/results/single-player and pause.
   Monster births, heavy entries and held aerial entries do not prove
   later callbacks. CPU audit `target/wave16/cpu-next-search/plan.md` finds
   no eligible new CPU branch in existing complete recordings; human/Nana
   data is not configured CPU control evidence. Nothing is out of scope.
   **No new recordings are authorized in the current task**; a concrete
   scratch recording pack is prepared, with permission pending.
5. [ ] Replay depth after breadth: five existing stops remain in the second
   corpus (Kirby/Game & Watch aerial Fire near FoD's edge; two proven
   unconsumed controller polls; two one-ulp Nana positions). Other
   unfinished games are refused capabilities. Preserve the distinction
   between replay stops, exact retail witnesses and unsupported setups
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
barrel; other ownership contexts remain guarded. Party Ball now has bounded neutral left-facing FALL/REST coverage;
its other states, other random kinds and heavy fighter rows remain backlog. Entry-only recordings do not
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
