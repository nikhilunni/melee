# Tracker

Legend: `[ ]` todo, `[~]` in progress, `[x]` done, `[!]` blocked.
Keep current facts and next work here; earlier checkpoints live in git history
and `docs/PORT_NOTES/`.

## Current state (2026-10-24, wave 14 integrated)

- **All six handoff branches are on main at `09b60d97`:** grab/common
  edges; Kirby's Nayru's Love, Fire Breath and Ice Shot; match flow;
  random items and Capsule/Box/Barrel; Kongo Jungle/Jungle Japes; and
  guarded Versus CPU players. The incoming notes in `docs/handoffs/` are
  historical. Integration evidence is under `target/wave14/`; the acceptance summary
  is `docs/PORT_NOTES/WAVE14_INTEGRATION.md`.
- **Full sweep:** 3,485 scenarios, **3,255 particle-exact**, 115 not
  comparable, 114 existing local failures and one existing particle fault.
  **Zero regressions, 193 fixes** against startup `0a99db31`. A local
  failure or incomplete recording is not an accepted divergence in a
  registered witness.
- **Replay corpora:** `public-v3.7` completes **108/108**, with 964,782
  matched frames; `public-v3.7-b` completes **486/502**, with 5,102,652
  matched frames. Every startup completion and matched frame is retained;
  the second corpus gains two completions and 21,572 frames. Unsupported
  stock-sharing, CPU replay setup and other unproved capabilities remain
  refused; Slippi does not record the CPU level needed for admission.
- **Parity inventory** (`target/parity/wave14-integrated/`,
  `docs/PARITY.md`): all retail **29.2% ported/partial, 37.9% with body/row**;
  historical in-game subtotal **44.9% / 56.9%**. Of 19,827 functions:
  4,200 ported, 204 partial, 1,361 body, 1,378 row, 221 referenced and
  12,463 not started. **253 fail-closed sites**: 234 unimplemented and 19
  bail sites. These citation/byte estimates are not a retail-proof score.
- **Combined release checkpoint at `bf98d61d`:** 1,935 workspace tests
  pass, zero fail, one existing ignore; m4 345, m5 295, API allocation 13.
  Release workspace clippy is clean and harness pytest passes 407 tests.
  WASM pure suites and all four guarded spline subprocess checks pass;
  native/WASM match hashes are identical over 6,003 lines. The release web
  check and refreshed inventory also pass. Schema checks report no layout
  violations. Release builds only;
  no debug checkpoint is claimed.
- **Performance gate remains blocked on user-owned budgets:** time passes
  (18.618 ms per 600 ticks; 57.191 ms load), while stripped size 7,605,760
  bytes and text size 6,848,512 bytes exceed their ceilings. `ft-kirby`
  has one duplicate label against baseline zero. No thresholds or
  baselines have been raised; user approval remains pending (`docs/PERF.md`).

## Next (breadth before individual replay fixes)

1. [x] Close the six-branch integration checkpoint at runtime source
   `bf98d61d`. Full-sweep/replay baseline and provenance are published under
   `target/sweeps/bf98d61d227f916fd01ab5b74a8f94ad20dde183.*`. Completed
   integration worktrees have been removed.
2. [~] **Ground Sword input/entry:** 23 existing complete witnesses:
   six side C-stick throws, seven vertical-smash routing cases, seven
   down-tilt routing cases and three direct up-smash entries. Only held
   Sword **AttackLw3 (57)** and **AttackHi4 (63)** admissions are proposed.
   Held aerials, LandingAir, AttackHi3 and AttackLw4 remain guarded. The
   25 aerial traces end at entry and do not prove the missing aerial
   throw-first IASA call. Plan: `target/wave14/held-sword-implementation-plan.md`.
3. [~] **Barrel ownership:** give an unowned REST barrel the primary
   fighter owner of the article that struck it; preserve secondary
   owner, team and stale attribution boundaries. Use the existing retail
   witnesses and the typed owner representation, not item-id-as-player.
4. [~] **Container explosions:** preserve the originating scheduler phase
   across deferred Resume, then prove empty Box/Barrel blasts against
   complete existing captures. Keep ownership changes separate.
   **Party Ball follows:** FALL/REST from the existing 600-tick left-facing
   capture; opening/content branches remain guarded.
5. [ ] Continue the breadth inventory: heavy pickup/carry/throws and other
   item-use rows, remaining random item kinds, **19 remaining Versus
   stages**, Kirby's copied Sheik ability, wider CPU stage awareness,
   stamina/coin/handicap/time modes, retail menus/results/single-player,
   pause and the rest of the fail-closed backlog. Nothing is out of scope.
   **No new recordings are authorized in the current task**; use existing
   protected data in place and keep unwitnessed branches guarded.
6. [ ] Replay depth after breadth: five existing stops remain in the second
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
Sword swings are ported. Random spawning and Capsule/Box/Barrel are
witnessed for two-player FD/Battlefield setups. Other setups, Party Ball,
other random kinds, heavy fighter rows and unwitnessed container blasts
remain guarded. Entry-only recordings do not prove later callbacks.

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
