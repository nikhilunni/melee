# Tracker

Legend: `[ ]` todo, `[~]` in progress (add date), `[x]` done, `[!]` blocked
(say on what). Keep this file short and current: it states what is true now
and what is next. Finished work lives in git history and `docs/PORT_NOTES/`;
do not keep a session log here.

## Current state (2026-09-28)

- **20 of 26 characters and all six tournament stages run bit-exact.**
  Registered: Fox, Falco, Marth, Roy, Captain Falcon, Ganondorf, Peach,
  Yoshi, Jigglypuff, Pikachu, Pichu, Mario, Dr. Mario, Luigi, Samus, Sheik,
  Zelda, Ice Climbers (Popo and Nana), Link, Young Link. Stages: Final
  Destination, Battlefield, Dream Land, Yoshi's Story, Fountain of Dreams,
  Pokémon Stadium. Each character has a start boundary vs Fox on FD and
  explorer batches; ten cross-matchup boundaries also run exactly.
- Fox vs Marth on FD is complete, including Sudden Death, with exit evidence
  in `docs/MATCHUP_COMPLETENESS.md`.
- Wave C of parallel agents finished on 2026-09-28; the user paused after it.
- Gates at the wave C checkpoint: release workspace 1641/0 after two fixes,
  `m5_gate` 266/0, clippy clean, pytest 265, schema check clean. The perf
  gate fails on size (see Next, item 5).
- Tooling:
  - `make_boundary.py` creates any stage and character boundary;
  - `explore_batch.py --boundary` runs the explorer from it;
  - `record_many.py` records in parallel;
  - `melee-sim search` and `triage`;
  - `MELEE_DATA_ROOT` lets worktrees use the main checkout's data;
  - `docs/AGENT_BRIEF.md` briefs worktree agents;
  - `tools/agent-merge/pick.sh` merges their commits.

## Next (recommended order)

1. [ ] **Slippi replay corpus (milestone 7).** Most real games now use
   registered characters and stages. Needs:
   - a local replay corpus;
   - a batch runner over `melee-sim replay` that groups first divergences
     with `triage`;
   - support for Slippi Online's initialization and seed resets (the current
     fixtures stop there);
   - a check of the `self_vel`/`kb_vel` field mapping;
   - Stadium read timing, possibly from Slippi 3.18's transformation event
     (0x41).

   CPU-controlled ports and non-stock rules stay out of scope.
2. [ ] **Close the open items.** Each fails closed or is unregistered:
   - wall/ledge tether for Samus and Link (AirCatchHit);
   - Ice Climbers: Squall Hammer, Belay, Blizzard, and Nana's death rules;
   - Peach's rare pulls (Beam Sword, Bob-omb);
   - Yoshi's delayed egg powershield;
   - Pichu: a bolt ending in Walk, and a Fox laser glancing Pichu
     (e68e14c22_p1);
   - Peach's down throw on Marth (one extra hitlag frame);
   - Mario's cape vs Falco's blaster;
   - Samus's up throw (samus_grab_throwu);
   - Ganondorf's floor snap (mpColl_800477E0) and the Dark Dive throw
     effect order;
   - Sheik crash after a transform (e255c070a_p0);
   - Yoshi turnaround forward-smash dust positions.

   Each item suits one worktree agent.
3. [ ] **The remaining six characters**, in rough order of difficulty:
   - Bowser;
   - Donkey Kong (cargo carry);
   - Mr. Game & Watch;
   - Ness (PK Thunder, yo-yo);
   - Mewtwo;
   - Kirby (copy abilities need every copied character's specials).

   Each goes through the bring-up recipe in `docs/AGENT_BRIEF.md`.
4. [ ] **Particle positions in the gate.** The gate checks particle RNG
   order and generators, not positions. Positions drift by small amounts in
   some long explorer matches (e.g. 6 of 8 PS matches, mario e9943b4ab_p0)
   and in some Mario/Falcon throw effects (±0.09, facing). Plan: add a
   per-tick particle digest to the gate, then fix what it finds.
5. [ ] **Perf duplicate-label census.** The size baseline was raised after
   wave C (user, 2026-09-28). The perf gate still fails its zero-tolerance
   duplicate-label check:
   - ft-iceclimbers: 35 labels (Popo and Nana instantiations);
   - one label in each new character crate;
   - melee-ft: 22 against 20;
   - across crates: 118 against 100.

   Either de-duplicate the labels or have the user re-baseline the census.
   Throughput is 30,188 ticks/s (last pass 32,438).

For a new matchup or stage, reuse the Fox-Marth approach:
- an exit-criteria table like `docs/MATCHUP_COMPLETENESS.md`;
- `interaction_matrix.py --stage --characters`;
- explorer batches from its boundaries;
- directed witnesses via `search`.

## Status by area

**Characters** (`crates/ft-<name>`):
- 20 of 26 are registered, with their specials; exceptions are listed in
  Next, item 2.
- Family crates: fox, mars, mario, pikachu, captain, link.
- Two-fighter players share one model: Ice Climbers, and Sheik/Zelda (the
  sleeping form).
- Unregistered: Kirby, Ness, Mewtwo, Mr. Game & Watch, Donkey Kong, Bowser.
  Their scenes fail at load.

**Stages**: all six tournament stages run explorer matches and long witnesses
exactly. This covers Yoshi's Story's Shy Guys and slopes, Fountain of Dreams'
moving platforms and water, and every Pokémon Stadium transformation. On
Stadium, the form-archive read completion is an external input
(`melee_lib::ExternalEvents`):
- recorded traces replay it;
- recordings keep it;
- otherwise a documented default latency table applies.

**Items**: the core item system and one `it-<kind>` crate per character's
items are ported:
- Fox and Falco: lasers and Illusion;
- Mario, Dr. Mario and Luigi: fireballs, pills and cape;
- Peach: parasol, Toad, spores, turnips, bomber blast;
- Yoshi: egg and star;
- Pikachu and Pichu: Thunder Jolt and Thunder;
- Samus: missiles, charge shot, bombs, grapple;
- Link and Young Link: bombs, arrows, boomerang, hookshot, milk;
- Sheik: needles, chain, Vanish;
- Zelda: Din's Fire;
- Ice Climbers: ice;
- Bob-omb;
- Yoshi's Story's Shy Guys.

Random items are unstarted.

**CPU** (`melee-cpu`): only Nana's follow logic. CPU players are out of scope.

**Out of the gate by design**: the in-game Start pause, menus, results and
single-player modes.

**Native app** (`melee-platform`): all planned rendering features exist.
Retail pixel fidelity, exact camera tracking and GX rounding quirks remain.
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
| 2026-09-28 | Subagents allowed on Opus 5.5 only, run widely in parallel; no Codex. |
| 2026-09-28 | Perf size baseline raised to 6,444,304 stripped bytes after wave C (user). |
| 2026-09-28 | Asynchronous disc reads that affect gameplay (Pokémon Stadium's forms) are external inputs, recorded from retail and replayed like pads; standalone runs use a documented default. |
