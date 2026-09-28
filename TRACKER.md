# Tracker

Legend: `[ ]` todo, `[~]` in progress (add date), `[x]` done, `[!]` blocked
(say on what). Keep this file short and current: it states what is true now
and what is next. Finished work lives in git history and `docs/PORT_NOTES/`;
do not keep a session log here.

## Current state (2026-09-28)

- **Fox vs Marth on Final Destination is complete**, including Sudden Death
  Bob-omb rain. Two recorded human matches replay bit-exact end to end, the
  interaction matrix (`docs/INTERACTION_MATRIX.md`) has no open rows, and the
  explorer finds no faults in its latest batches (normal and Sudden Death).
  Exit evidence: `docs/MATCHUP_COMPLETENESS.md`.
- Gates: debug 1526/0, release 1527/0, clippy clean, perf gate PASS (size
  baseline raised by the user on 2026-09-28, `docs/PERF.md`).
- Tooling: `explore_batch.py`, `record_many.py`, `melee-sim search`,
  `melee-sim triage`, `make_boundary.py` (see "Verification workflow" in
  `CLAUDE.md`). Going wide is set up: `make_boundary.py` creates, gates and
  registers a retail start boundary for any stage and character pair in about
  30 s; the explorer and `explore_batch.py` take `--boundary`;
  `interaction_matrix.py` takes `--stage` and `--characters`.

## Next (user to choose; recommended order)

1. [ ] **Slippi Fox-vs-Marth on FD.** Replay real tournament games through
   `melee-sim replay`; build the batch runner that aggregates first
   divergences, using `triage` for the reports. Verify the Slippi
   `self_vel`/`kb_vel` field mapping first.
2. [ ] **Battlefield.** Most real games are not on FD, and platforms open the
   branches marked n/a on FD (pass-through, shield break on a platform, the
   other shield-break orientation, DownReflect). Boundaries
   `start_bf_fox_marth4` and `start_bf_marth_fox4` are registered; a first
   explorer run (2026-09-27, 5 seeds) faults on `gm_80167638` (revival
   marker offset), `ftCo_Pass.c:56-60` (shield platform drop),
   `ftCo_8009A134` (Fire Fox platform skip),
   `ftFx_SpecialLwStart_CheckPass` and `grbattle.c:379-380`.
3. [ ] **Falco** (shares `ft-fox-family`), then other characters ranked by
   tournament usage. Boundaries `start_fd_falco_fox4` and
   `start_fd_fox_falco4` are registered; the first run faults on a missing
   animation-table entry (`spawn.rs:1114`, an unnamed map lookup: name it) and
   `ftCo_800DEA28` (taunt).

For a new matchup or stage, reuse the Fox-Marth approach: an exit-criteria
table like `docs/MATCHUP_COMPLETENESS.md`, a matrix from
`interaction_matrix.py --stage <Stage> --characters ...`, explorer batches
from its boundaries (`explore_batch.py --boundary ...`) with bridged samples,
and directed witnesses via `search`. Independent faults suit one worktree
agent each (`CLAUDE.md` "Agents").

## Status by area

**Characters** (`crates/ft-<name>`): Fox and Marth complete for the matchup.
Falco, Captain Falcon, Peach, Jigglypuff and Yoshi pass idle, start and
movement scenes only. Mario's crate is a stub and not registered in
`scene_characters!` (his recorded scenes fail to load). All others unstarted.

**Stages**: Final Destination complete. Battlefield passes Fox idle/start and
platform scenes and the Fox/Marth start boundaries. Yoshi's Story, Dream Land and Fountain of Dreams have idle and
start recordings: Yoshi's Story diverges at tick 13 (the stage's Shy Guy item,
kind 210, is not spawned), Fountain of Dreams is not supported by the port, and
Dream Land (Whispy wind, Bronto Burts) is unported.

**Items**: the core item system, Fox laser and Illusion, and Bob-omb are
ported. Other items are unstarted.

**Out of the gate by design**: the in-game Start pause (not modelled), menus,
results and single-player modes; CPU AI (`melee-cpu` is a stub).

**Native app** (`melee-platform`): all planned rendering features exist; retail
pixel fidelity, exact camera tracking and GX rounding quirks remain.

## Backlog

- [ ] CI: `cargo gate`, clippy, harness pytest, `gen_schema.py --check`.
- [ ] 109 `unimplemented!` boundaries remain (26 in `melee-ft`): each is a
      branch no gated scenario reaches; port them as explorer faults or new
      content reach them.
- [ ] `melee-cpu`: CPU AI (`ftCo_0A01.c`, `ftcpuattack.c`, `ftcmdscript.c`).

## Blockers

None.

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
| 2026-09-09 | Motion-state tables, static character tables, family crates, concrete core and shell, no per-tick allocation (architecture rules in `CLAUDE.md`). |
| 2026-09-26 | Record with headless Dolphin by default; windowed only for human play and menu driving. |
| 2026-09-26 | The explorer corpus starts from registered retail boundaries, so every case is replayable in Dolphin. |
| 2026-09-28 | Subagents allowed on Opus 5.5 only, at most two building at once; no Codex. |
| 2026-09-28 | Perf size baseline raised to 4,956,208 stripped bytes (user). |
