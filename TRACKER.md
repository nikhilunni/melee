# Tracker

Legend: `[ ]` todo, `[~]` in progress, `[x]` done, `[!]` blocked.
Keep current facts and next work here; earlier checkpoints live in git history
and `docs/PORT_NOTES/`.

## Current state (2026-10-25, wave 22 integrated)

- **Held Sword F66/Hi68 entry and coupled Klaptrap birth are integrated at
  checkpoint source `f1288878`:** thirteen original whole Sword captures
  (12,633 ticks) and three Klaptrap captures (4,998 ticks) match through
  actual entry/birth and remaining same-tick phases. Existing back-air and
  midair catches stay exact. Klaptrap includes its real path/tangent/Euler
  work, live parent, pose/script/hitbox and stage-bank particle; next actual
  Ground Animation/item Animation and unsupported contexts stay guarded.
  A source/data-proven Giant Punch hitbox invariant adds no gameplay
  behavior. Details: `docs/PORT_NOTES/WAVE22_BREADTH.md`.
- **Fresh combined sweep:** 3,490 scenarios, **3,348 particle-exact**, 115
  not comparable, 26 existing local failures and one existing particle
  fault. **Zero regressions, sixteen fixes** against `6d0c850e`; **281 fixes**
  against startup `0a99db31`. Incomplete recordings and local failures are
  not accepted divergences. Source, executable and scenario provenance are
  under `target/wave22/main/`; canonical fresh baseline is
  `target/sweeps/f1288878be775815aa6b31c39d1b2c8586a703b6.*`.
- **Fresh replay corpora remain unchanged:** `public-v3.7` completes
  **108/108** (964,782 matched frames); `public-v3.7-b` completes **486/502**
  (5,102,652 frames), preserving every path/category/matched/total-frame
  tuple. CPU replay setup and stock sharing remain refused capabilities.
- **Parity inventory** (`target/parity/wave22-integrated/`,
  `docs/PARITY.md`): all retail **29.4% ported/partial, 38.1% with body/row**;
  historical in-game subtotal **45.2% / 57.2%**. Of 19,827 functions:
  4,211 ported, 225 partial, 1,372 body, 1,381 row, 231 referenced and
  12,407 not started. **299 fail-closed sites**: 280 unimplemented, 19 bail.
  Ported plus partial grows by twelve functions; net thirteen additional
  guards expose unsupported contexts. Citation counts are not witnessed
  branch coverage; every reporting group remains in scope.
- **Combined release checkpoint passes:** 2,006 workspace tests, zero
  failures, one existing ignore; m4 345 and m5 312. Workspace all-target
  clippy is clean; harness 407 and schema zero layout violations. WASM pure
  suites, **five expected spline traps**, 6,003 identical native/WASM hash
  lines and the release web check pass. All nine worktree data-path failures
  pass in MAIN without optout. Actual API step/observe/capture is allocation-
  free with independent before-entry clones; clone creation is outside
  measured windows. Klaptrap's matrix proof is only each recorded birth
  DFS1 SRT and twelve world-matrix words, not whole bones/cache/pixels.
  Release only; no debug run.
- **Quiet performance retains existing budget failures:** at `f1288878`,
  time passes (18.417 ms per 600 ticks; 55.485 ms load). Stripped size is
  7,675,632 bytes and text size 6,897,664 bytes, above unchanged ceilings;
  Kirby's one duplicate label still exceeds baseline zero. Shared-code
  duplication passes: 20 melee-ft labels, 94 across crates. Stripped size
  grows by 1,248 bytes versus wave 21; text is unchanged. Full report is
  appended to `docs/PERF.md`; no budget was increased.

## Next (breadth before individual replay fixes)

1. [x] **Shared ordinary-contact handling:** both complete 360/699-tick
   captures, genuine phantom controls, raw state, API and full checkpoint
   pass. The item analogue now refuses unsupported mixed early returns.
2. [x] **HeavyGet terminal entry:** five original complete captures, raw
   scratch/flags/ownership, genuine RNG and strict whole particles pass.
   Next Animation/IASA, attachment, carry and throws remain guarded.
3. [x] **Item mixed-contact refusal:** source-proven missing-clear
   early-return paths now refuse until witnessed. Four complete ordinary
   item controls remain exact. The bounded 45-scene search found no eligible
   whole mixed-contact witness; this change admits no item behavior.
4. [x] **Kongo log-standing:** complete 2,829-tick warm/cold witness;
   Sink/Remove and second-live-log paths remain guarded.
5. [x] **Revival-floor entry:** complete 1,679-tick recording, with the
   actual floor transition on its last tick. No later floor-exit proof.
6. [!] **Ness current yo-yo binding:** raw data proves overlapping old/new
   instances, but the only overlap capture has 4,775 expected/raw/ledger
   ticks and only 3,340 particle/meta ticks. Whole acceptance requires an
   add-only new recording; approval remains pending. Do not truncate it.
   Evidence: `target/wave18/yoyo-witness-search/plan.md`.
7. [~] **Next bounded port:** common held Sword Down Air entry, using five
   original whole captures. A semantic static character-table capability
   must preserve both Link/YoungLink's normal entry and direct bounce
   restart refusals; private rows, midair catches and immutable entry
   ownership remain distinct. F/B/Hi entries and bounded Klaptrap/monster
   births are integrated; later callbacks stay guarded. Continue heavy
   carry/throws, item-use rows and random kinds, **19 remaining Versus
   stages**, copied Sheik Needle Storm, wider CPU stage awareness, modes,
   retail menus/results/single-player and pause. CPU audit found no eligible
   additional branch in existing complete configured-CPU recordings.
   Nothing is out of scope. **No new recordings are authorized in the
   current task**; the concrete scratch recording pack awaits permission.
   Second-log data ends at creation without coexistence/lifecycle proof.
8. [ ] Replay depth after breadth: five existing stops remain in the second
   corpus (Kirby/Game & Watch aerial Fire near FoD's edge; two proven
   unconsumed controller polls; two one-ulp Nana positions). Other
   unfinished games are refused capabilities. Preserve replay/retail
   witness/unsupported-setup distinctions (`docs/SLIPPI.md`).

## Status by area

**Characters:** all 26 are registered. Movement, combat and character
specials have recorded witness families. Kirby has **24/25 copied neutral
specials**; copied Sheik Needle Storm remains unported despite an existing
start boundary (`docs/PORT_NOTES/KIRBY.md`). Pinned mouth-victim bottom cleanup is witnessed; other Inhale/item and
hand-pose gaps remain. Character differences use static tables and family crates.

**Stages:** ten supported Versus stages: the six tournament stages, Yoshi's
Island 64, Kongo Jungle 64, Kongo Jungle and Jungle Japes. Supported does
not mean every hazard is enabled: Kongo barrel/log Sink/Remove, second-log and other
unwitnessed branches retain explicit guards. Klaptrap admits only its real
coupled birth tick; next Ground Animation/item lifecycle remains guarded. Nineteen stages remain.
Stadium form-archive read completion is a recorded external input; ordinary
runs use documented default latency (`docs/APP.md`).

**Items:** shared engine and character articles, Peach's rare pulls and
Sword swings are ported. Random spawning and Capsule/Box/Barrel, including
empty blasts, are witnessed for two-player FD/Battlefield setups. Ground
Sword admissions add only common AttackLw3 (57), AttackHi4 (63) and their
witnessed throw routing. Common F/B/Hi aerials additionally admit only
recorded held entry; neutral/Down/private aerials, Hi3 and Lw4 stay guarded.
Barrel ownership adds only a primary article owner to an unowned REST
barrel; other ownership contexts remain guarded. Party Ball now has bounded neutral left-facing FALL/REST coverage;
its other states, other random kinds and later heavy fighter rows remain backlog.
HeavyGet terminal entry is integrated, with no ownership/carry continuation claim.
Running Barrel scripts retain their original program across witnessed rolling
drops; this adds no new item kind or heavy/aerial fighter admission. Entry-only recordings do not
prove later callbacks. Kuriboh and left-facing ReDead constructors and the
coupled Klaptrap birth slice add only their explicit witnessed contexts.

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
