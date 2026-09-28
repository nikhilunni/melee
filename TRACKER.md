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

1. [~] **Going wide (2026-09-27, parallel agents).** Six stages run Fox vs
   Marth exactly (boundaries, long witnesses, explorer matches): Final
   Destination, Battlefield, Dream Land, Yoshi's Story, Fountain of Dreams,
   Pokémon Stadium. Every registered character has its specials ported:
   Fox, Falco, Marth, Captain Falcon, Peach, Yoshi, Jigglypuff, Pikachu (a
   kind without ported specials fails closed on B). Registered boundaries: FD/BF/DL/YS/FoD/PS
   Fox-Marth, and each other character vs Fox on FD. Open:
   - **User decision: Pokémon Stadium transformations depend on emulated
     disc read latency**, which varies between recordings even for a match's
     first form read (21-23 polls for the same file). The port uses a
     measured table for the first read (exact on the registered witnesses)
     and fails closed on later reads; explorer matches are exact only up to
     the first transformation. Options: model Dolphin's disc timing, or take
     the read-completion tick from the recording as an input.
     docs/PORT_NOTES/POKEMON_STADIUM.md.
   - In flight: Mario bring-up (stub crate).
2. [ ] **Slippi.** Replay real tournament games through `melee-sim replay`;
   build the batch runner that aggregates first divergences, using `triage`.
   Verify the Slippi `self_vel`/`kb_vel` field mapping first. Needs a local
   replay corpus.
3. [ ] More characters by tournament usage (Sheik, Samus, Ice Climbers,
   Pikachu, Luigi, Dr. Mario, Ganondorf, Link, Mario...): `make_boundary.py`
   then explore; unregistered kinds fail at load.

For a new matchup or stage, reuse the Fox-Marth approach: an exit-criteria
table like `docs/MATCHUP_COMPLETENESS.md`, a matrix from
`interaction_matrix.py --stage <Stage> --characters ...`, explorer batches
from its boundaries (`explore_batch.py --boundary ...`) with bridged samples,
and directed witnesses via `search`. Independent faults suit one worktree
agent each (`CLAUDE.md` "Agents").

## Status by area

**Characters** (`crates/ft-<name>`): Fox and Marth complete for the matchup.
Falco, Captain Falcon, Peach, Yoshi, Jigglypuff and Pikachu (with
`ft-pikachu-family` for Pichu) have every special ported and pass their start
boundaries and explorer batches. Mario is in flight; all other kinds are
unregistered. Mario's crate is a stub and not registered in
`scene_characters!` (his recorded scenes fail to load). All others unstarted.

**Stages**: Final Destination complete. Battlefield, Dream Land, Yoshi's Story
(Shy Guys, slopes), Fountain of Dreams (moving platforms, water terrain) and
Pokémon Stadium (all four forms; later transformations fail closed on disc
read latency) run explorer matches and long witnesses exactly.

**Items**: the core item system, Fox laser and Illusion, Bob-omb, Yoshi's
egg and star, Peach's parasol, Toad, spores, bomber blast and turnip, and
Yoshi's Story's Shy Guys are ported. Random items are unstarted.

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
