# Tracker

Legend: `[ ]` todo, `[~]` in progress, `[x]` done, `[!]` blocked.
Keep current facts and next work here; earlier checkpoints live in git history
and `docs/PORT_NOTES/`.

## Current state (2026-10-26, wave 27 closed at `ffcfe8a8`)

- **Wave 27 (seven concurrent agents; `target/wave27/`) landed on main:**
  **eleven more stages** (Brinstar, Venom, Mute City, Poké Floats, Mushroom
  Kingdom II, Great Bay, Icicle Mountain, Princess Peach's Castle, Green
  Greens, Rainbow Cruise, Big Blue: **28 supported**; Corneria is on a WIP
  branch); random items Ray Gun, Fire Flower, Super Scope, Starman, Cloaking
  Device, Screw Attack, Metal Box, Bunny Hood (not yet in the pool), the
  Warp Star ride, Lip's Stick, Parasol, the Party Ball and the Goomba (pool:
  18 kinds); items with 2-4 players on the wave-26 stages; CPUs with up to
  four players on FD/Battlefield; Coin mode and Sudden Death among only
  the tied players; full gamepad parity in both apps (API 6). Shared fixes:
  item hits honour x42_b6 (grabbable hurtboxes only), the item spin joint
  from ItemModelDesc xC, a death's motion change runs ChangeMotionState's
  tail, star/screen KOs ride moving floors, unreachable app seeds retry (`StartError::UnreachableSeed`).
  Details: `docs/PORT_NOTES/` (WAVE27_*, VERSUS_RULES, CPU_PLAYERS,
  RANDOM_ITEM_KINDS).
- **Gates:** at `329c16e2` the full sweep had **0 regressions** against
  `e7e3a37d` (61 fixed); `m4_gate` 364 and `m5_gate` 321 pass at
  `208d7a84`, with `melee-lib` and `melee-platform`; every merged branch
  reported `public-v3.7` 108/108 (964,782 frames). Closing checkpoint at
  `56abf9e7`: `public-v3.7` **108/108**; sweep 6 regressions (Yoshi's Story
  CPU and Ice Climbers' Nana) and Green Greens' start boundary failing, all
  from the CPU stages merge's savestate island-order restore, so that merge
  is **reverted** (`ffcfe8a8`); after the revert the six and the boundary
  gate exact and `m4_gate` passes 365. Big Blue had no regression. Not
  rerun: full `m5_gate` after the revert, debug gate, `public-v3.7-b`, perf
  gate, parity inventory.
- **Disk:** traces are ~205 GB; the wave filled the disk once. Engrams
  worktrees were removed (user, 2026-10-26; 21 dirty ones stashed as
  `cleanup-2026-10-26:*` in engrams). Keep `record_many --jobs 1-2`.
- **Performance:** last measured at `7c06907b`; not rerun.
- **New captures are authorized (user, 2026-10-25):** fresh names, keep
  existing data and failed attempts, never commit game data.

## Next (breadth before individual replay fixes)

1. [ ] **Verify the closing checkpoint** (above), then the WIP branches,
   each with a note in `docs/handoffs/`:
   - `worktree-agent-a1c35ef88761f969e` WIP `b61da852`: Corneria, items on
     Brinstar/Venom/Corneria (`wave27-corneria.md`); gated before its last
     rebase only.
   - `worktree-agent-a04cc103feaee3dd4` WIP `a61acfff`: Hammer, Super/Poison
     Mushrooms, Bunny Hood explorer clean (`hammer-mushrooms-wave27.md`);
     not rebased; one Hammer head ulp divergence.
   - `worktree-agent-aa20640d8986a1449` WIP `ffe1f6dc`: Mushroom Kingdom `?`
     blocks, Kongo guards; stopped mid-debug, unverified.
   - `worktree-agent-a2e9b841e1b181e99` WIP `6227e7a6`: two Big Blue
     scenarios to gate (`wave27-big-blue.md`).
   - `worktree-agent-a00a7da3dbc67e2bf` (`b2936f4a`, `6fbf6217`): CPUs on
     MK2, Great Bay, Brinstar, Mute City, Poké Floats; **reverted from main**:
     its import island-order restore broke Yoshi's Story CPU/Nana and Green
     Greens' saved islands. Fix that, then Icicle Mountain and Venom's two
     divergences (`w27-stage-cpus.md` on the branch).
2. [ ] **Items:** Poké Ball and its Pokémon, Flipper, Barrel Cannon
   (`WAVE27_RIDE_ITEMS.md`); items on the wave-27 stages; Bunny Hood into the
   pool.
3. [ ] **Modes:** Camera mode (`VERSUS_RULES.md`); Coin with items on.
4. [ ] **Open divergences** (`WAVE26_FOLLOWUPS.md`): Mushroom Kingdom 0.0015
   at tick 238, Onett one ulp at 3745, Kongo's second log; the
   ftCo_800DC920 captured-fighter death guard (hit on several stages).
5. [ ] Replay depth after breadth: five stops in `public-v3.7-b`.

## Status by area

**Characters:** all 26 are registered, with recorded witness families for
movement, combat and specials. Kirby has **all 25 copied neutral specials**
(`docs/PORT_NOTES/KIRBY.md`); other Inhale/item and hand-pose gaps remain.
Character differences use static tables and family crates.

**Stages:** 28 supported Versus stages: wave 26's 17 plus Brinstar, Venom,
Mute City, Poké Floats, Mushroom Kingdom II, Great Bay, Icicle Mountain,
Princess Peach's Castle, Green Greens, Rainbow Cruise and Big Blue
(`docs/PORT_NOTES/WAVE26_STAGES.md`, `WAVE27_*`). Corneria is on a WIP
branch. Supported does not mean every branch is: rare hazards and
mid-hazard savestates stay guarded.
Stadium form-archive read completion is a recorded external input.

**Items:** shared engine, character articles and Peach's rare pulls; random
spawning with Capsule, Box, Barrel and Party Ball (fall/rest); swing items
(Beam Sword, Home-Run Bat, Fan, Star Rod); heavy carry and throws; Green and
Red Shells, Freezie, Bob-omb, Mr. Saturn, Motion-Sensor Bomb, Heart
Container, Maxim Tomato, Food, Warp Star (fall/rest); ReDead with
CaptureLeadead; fighters stomp items (`docs/PORT_NOTES/RANDOM_ITEMS.md`).
Wave 27 added the shooters, Starman, Cloaking Device, Screw Attack, Metal
Box, Bunny Hood, the Warp Star ride, Lip's Stick, Parasol and the Goomba.
Items run with 2-4 players on the wave-26 stages except Mushroom Kingdom;
the wave-27 stages refuse items until witnessed.

**CPU:** Versus CPU players on wave 26's 17 stages (`Stage::cpu_supported()`);
up to four players on FD/Battlefield, two elsewhere
(`Stage::cpu_max_players`). The wave-27 stages' CPUs are on a reverted
branch. CPU replay admission is still refused
because Slippi lacks CPU levels.

**Match flow:** FFA, teams/doubles, the team stock steal, Time (two-player
Sudden Death), Stamina, handicap, Damage Ratio and the Special Melee modes
except Camera; Coin mode (items off) and Sudden Death among the tied
players (`docs/PORT_NOTES/VERSUS_RULES.md`).

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
