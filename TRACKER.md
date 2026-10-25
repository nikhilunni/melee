# Tracker

Legend: `[ ]` todo, `[~]` in progress, `[x]` done, `[!]` blocked.
Keep current facts and next work here; earlier checkpoints live in git history
and `docs/PORT_NOTES/`.

## Current state (2026-10-25, wave 26 closed at `e7e3a37d`)

- **Wave 26 (six, then five, concurrent agents; `target/wave26/`) landed on
  main:** Ness's current yo-yo binding; Kirby's copied Needle Storm (**all 25
  copied specials**); heavy items carried and thrown (HeavyGet, Lift*, all
  eight heavy throws); ReDead's lifecycle and CaptureLeadead; the Beam Sword's
  remaining rows, Home-Run Bat, Fan and Star Rod; Green and Red Shells,
  Freezie, random Bob-omb and Mr. Saturn, the Motion-Sensor Bomb, Heart
  Container, Maxim Tomato, Food and the Warp Star's fall and rest, with
  fighters stomping items; **seven more stages** (Flat Zone, Hyrule Temple,
  Fourside, Brinstar Depths, Yoshi's Island, Mushroom Kingdom, Onett: 17
  supported); **CPU players on all 17 stages** with every recovery special;
  Time mode, team stock steal, handicap and Damage Ratio, timed teams,
  Stamina (with its KO), Lightning, Slo-Mo, Super Sudden Death, Single
  Button, Invisible, Giant and Tiny; and in both apps CPU players, four slots
  with teams, random items and these modes. Shared fixes: DownReflect out of
  DownDamage, no leg IK in hitlag, the repeated-hit push before the moving
  floor, mid-match allocations on four stages, the item attribute file read
  from ItCo.usd. Details: `docs/PORT_NOTES/` (WAVE26_STAGES, WAVE26_REDEAD,
  VERSUS_RULES, CPU_PLAYERS, FLAT_ZONE, RANDOM_ITEMS, KIRBY).
- **Gates at `e7e3a37d`:** sweep 4,062 scenarios, **3,751 exact**, 118 not
  comparable, **0 regressions** against `09deaa39` (77 fixed); the 193
  failing are local scenarios not on main (guarded stops, unmerged branches'
  recordings, unrecorded explorer TOMLs). `public-v3.7` replays **108/108**
  (checked at `9377e5cd`). `melee-lib` suite passes, including 28
  allocation tests; `melee-platform` tests and the wasm check pass. Not rerun
  this wave: the debug gate, full `m5_gate`, `public-v3.7-b`, perf gate.
- **Parity inventory** (`docs/PARITY.md`, rerun at `e7e3a37d`): all retail
  **32.0% ported/partial, 41.0% with body/row** (29.4 / 38.1 at wave 25);
  in-game **49.2% / 61.7%** (45.2 / 57.2). 4,635 ported, 275 partial, 11,696
  not started of 19,827. **366 fail-closed sites** (297): new code arrives with
  its unwitnessed branches guarded.
- **Performance:** last measured at `7c06907b` (19.084 ms per 600 ticks;
  user-approved size baselines 7,605,760 / 6,848,512). Not rerun after this
  wave's new crates.
- **New captures are authorized (user, 2026-10-25):** use fresh names,
  keep existing data and failed attempts, never commit game data. Record with
  `record_many.py --jobs 2` or less when agents share the machine: under
  heavy load recordings have differed between runs.

## Next (breadth before individual replay fixes)

1. [ ] **Merge the handed-off branches** (each has a note in
   `docs/handoffs/`): `app-guarded-paths.md` (Party Ball lifecycle, CPUs in
   3-4 player matches, items on every stage; one WIP commit to split, guard
   and register), `VERSUS_RULES.md` (Coin mode WIP, partial Sudden Death,
   Camera mode), `gamepads.md` (core done; macOS and web hosts unfinished;
   the user requires full pad-only parity), `random-item-wstar.md` (the Warp
   Star ride and spin joint), `WAVE26_MK2.md` (Mushroom Kingdom II plan).
2. [ ] **More stages:** Mushroom Kingdom II, Great Bay, Princess Peach's
   Castle, Rainbow Cruise, Brinstar, Corneria, Venom, Icicle Mountain, Green
   Greens, Poké Floats, Mute City, Big Blue (12 remain).
3. [ ] **More items:** Super/Poison Mushroom, Lip's Stick, Parasol item,
   Poké Ball and its Pokémon, Ray Gun, Fire Flower, Super Scope, Hammer,
   Screw Attack, Bunny Hood, Cloaking Device, Metal Box, Starman, Flipper,
   Barrel Cannon; the dropped Fan's landing (reads uninitialised xD4C).
4. [ ] **Open divergences** (`docs/handoffs/WAVE26_FOLLOWUPS.md`): Mushroom Kingdom
   0.0015 at tick 238, Onett one ulp at 3745, Kongo's second log.
5. [ ] Replay depth after breadth: five existing stops in `public-v3.7-b`
   (`docs/SLIPPI.md`).

## Status by area

**Characters:** all 26 are registered, with recorded witness families for
movement, combat and specials. Kirby has **all 25 copied neutral specials**
(`docs/PORT_NOTES/KIRBY.md`); other Inhale/item and hand-pose gaps remain.
Character differences use static tables and family crates.

**Stages:** 17 supported Versus stages: the six tournament stages, Yoshi's
Island 64, Kongo Jungle 64, Kongo Jungle, Jungle Japes, Hyrule Temple,
Fourside, Brinstar Depths, Flat Zone, Yoshi's Island, Mushroom Kingdom and
Onett (`docs/PORT_NOTES/WAVE26_STAGES.md`, `FLAT_ZONE.md`, `N64_STAGES.md`).
Supported does not mean every branch is: Kongo's second log, the Klaptrap's
later motions and a savestate taken mid-hazard stay guarded. Twelve remain.
Stadium form-archive read completion is a recorded external input.

**Items:** shared engine, character articles and Peach's rare pulls; random
spawning with Capsule, Box, Barrel and Party Ball (fall/rest); swing items
(Beam Sword, Home-Run Bat, Fan, Star Rod); heavy carry and throws; Green and
Red Shells, Freezie, Bob-omb, Mr. Saturn, Motion-Sensor Bomb, Heart
Container, Maxim Tomato, Food, Warp Star (fall/rest); ReDead with
CaptureLeadead; fighters stomp items (`docs/PORT_NOTES/RANDOM_ITEMS.md`).
Items are witnessed for two-player FD/Battlefield; more players and stages
are on a handoff branch.

**CPU:** Versus CPU players on all 17 stages, every recovery special, in
two-player matches (`Stage::cpu_supported()`, `CPU_MAX_PLAYERS = 2`); 3-4
player CPUs are on a handoff branch. CPU replay admission is still refused
because Slippi lacks CPU levels.

**Match flow:** FFA, teams/doubles, the team stock steal, Time (two-player
Sudden Death), Stamina, handicap, Damage Ratio and the Special Melee modes
except Camera; Coin mode is on a handoff branch (`docs/PORT_NOTES/VERSUS_RULES.md`).

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
- User-owned open decisions: performance budgets;
  Slippi Dolphin unfused math policy; default lag-frame inference; pushing
  main/first CI run. No push is authorized.
- Fresh retail branch coverage is separate from the citation inventory;
  see `docs/PORT_NOTES/RETAIL_COVERAGE.md`. Before assigning historical
  fault packets, verify current code, guards and available witnesses.

## Decisions still in force

| Date | Decision |
|---|---|
| 2026-10-25 | User approved size baselines 7,605,760 stripped / 6,848,512 text and Kirby duplicate-label allowance one; timing, other duplication limits and 5% size tolerance unchanged. |
| 2026-10-25 | Fresh captures and required new boundaries are authorized for 100% parity. No further capture approval is needed; preserve existing data and never commit game data. |
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
