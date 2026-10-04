# Tracker

Legend: `[ ]` todo, `[~]` in progress (add date), `[x]` done, `[!]` blocked
(say on what). Keep this file short and current: it states what is true now
and what is next. Finished work lives in git history and `docs/PORT_NOTES/`;
do not keep a session log here.

## Current state (2026-10-04, third Slippi wave and four characters)

- **25 of 26 characters and all six tournament stages run bit-exact.**
  Registered: Fox, Falco, Marth, Roy, Captain Falcon, Ganondorf, Peach,
  Yoshi, Jigglypuff, Pikachu, Pichu, Mario, Dr. Mario, Luigi, Samus, Sheik,
  Zelda, Ice Climbers, Link, Young Link, and new this wave Donkey Kong
  (cargo carry, Bury), Bowser, Mr. Game & Watch, Ness (with the yo-yo
  smashes) and Mewtwo. Only Kirby is unregistered. Each new character has
  a start boundary vs Fox on FD (`m4_gate`), a witness list in `m5_gate`
  and explorer matches.
- **Slippi replays** (every tick's Pre Frame seed is checked, so a complete
  game has no random-stream drift):
  - `~/melee-data/replays/public-v3.7` (108 games): **106 complete**, 98.4%
    of frames;
  - `~/melee-data/replays/public-v3.7-b` (502 games, sampled 2026-10-03;
    triage in its `_triage/` folder): **396 complete**, 83.7% of frames
    (247 when sampled); 48 are refused at setup (unregistered Kirby,
    online games, doubles, CPU players).
- **Console codes a replay does not record are read from its frames** by a
  dry pass that compares the candidate setups and then runs the whole
  comparison under one (never a switch mid-run; the report prints each
  choice; a flag overrides it): UCF 0.73/0.74/0.8/0.84 (bounded by the
  Slippi version, then the date, then the first dashback they disagree on),
  Dween (runs no fix: the recording holds its output), Frozen Stages
  (`--stage-codes`), Widescreen (`--screen`), the spawn rule (frame zero).
- **Not port faults** (documented in `docs/SLIPPI.md`): the off-screen
  magnifier's damage tick and some star-KO positions follow display passes a
  console skipped, which a replay does not record; a few Samus tether games
  where retail in Dolphin equals the port but the console drew otherwise.
- Gates at the checkpoint (2026-10-04): release workspace 1743/0 (incl.
  `m4_gate` 316 and `m5_gate` 286), clippy clean, pytest 301, schema check
  clean. **Perf gate: REGRESSION on size and duplicate labels** (see Next).
- Tooling (`CLAUDE.md` "Slippi replays", `docs/SLIPPI.md`):
  - `make_boundary.py`: any stage, characters, costumes; with
    `--no-register`, ports, Gecko codes, spawn rule, timer, and
    `--game-start-seed` (a per-replay boundary, needed on FD and FoD);
  - `record_many.py` with Gecko codes (`MELEE_GECKO_DIR=~/melee-data/gecko`);
    a scenario's `after_map = true` samples retail at the end of each
    fighter's map proc (Slippi before 3.4.0 samples there);
  - `melee-sim replay`, `replay-batch` (`pN.input_seed` = RNG drift on the
    tick before), and the replay-to-retail bridge (`--retail-inputs`,
    `harness/slippi_to_scenario.py`), which follows a replay on every stage;
  - `MELEE_DATA_ROOT` for worktrees; `docs/AGENT_BRIEF.md`;
    `tools/agent-merge/pick.sh` (array lengths in `m5_gate` lists often
    need fixing after a merge: compile the tests).

## Next (recommended order)

1. [!] **Perf gate (user decision).** 6,565,712 stripped bytes against the
   6,445,622 allowed (+5% over 6,138,688), still under the 6,766,519 fixed
   ceiling; time passes (19.3 ms / 600 ticks, 162 ms load). Duplicate
   labels: each new character crate has 1 (the same shared-default label
   every older character crate has, but no baseline entry), Mr. Game &
   Watch 2 (a `restore_saved` closure), `melee-ft` 22 against 20 (one is
   a `begin_damage_reaction` closure). Either raise the baselines for five
   new characters or reduce first; `docs/PERF.md` records the choice.
2. [~] **Slippi corpus.** Remaining stops in `public-v3.7-b`, by kind:
   - RNG drift with no shared cause left (each a separate fault): the
     hit-log victim keyed by GObj address (retail reuses a freed address;
     the port keys by unique id), Fox's wall tech while smoking from Fire
     Bird, Fox's Reflector hitting Link below the stage, PK Thunder's trail
     generator order, Ice Climbers explorer faults (Blizzard puffs, Ice
     Shot particles, a Nana behaviour-18 choice, a staleness difference);
   - Ice Climbers: Nana position and seed stops after KO credit;
   - six HNC Fountain of Dreams games created both side platforms level
     (code text not found; a scratch experiment matches two fully);
   - the 2019 Dream Land NeutralSpawn row has no Gecko text, so those games
     cannot be bridged; Stadium consoles that set per-player spawn points
     cannot be bridged until `--retail-inputs` carries the spawn point;
   - unported: a light hit on a grabbed fighter whose captor is launched
     (`ftCo_8008EC90`, Ness's PK Thunder), the sliding Samus bomb, Dr.
     Mario's neutral special while holding an item, a frozen fighter
     leaving the top, Mr. Saturn's idle and slide states.
   Then a newer corpus: Slippi Online (per-frame RNG sync, netplay codes)
   and Gecko-list (3.3+) code detection instead of inference.
3. [ ] **Kirby**, last: copy abilities need every copied character's
   specials, all of which now exist.
4. [ ] **Fail-closed branches left from wave D**: Captain Falcon's sword
   Swing42, item-on-item push, Mr. Saturn's idle and slide states, UCF 0.84
   branches without witnesses (KO totals, Nana's behaviours 5, 6 and 15,
   DamageIce, the sword swings and Mr. Saturn knocked away are done).
5. [ ] **Particle positions in the gate.** The gate checks particle RNG
   order and generators, not positions (wave D found one class of bug this
   way: sound draws out of script order). Add a per-tick particle digest to
   the gate, then fix what it finds.
6. [ ] **Tick-path allocations.** Dream Land re-decodes its background
   AnimJoint on about 60% of ticks
   (`BackgroundAnimation::select_animation`); extend the allocation test
   to every stage and character, then prepare stage animations at load.

For a new matchup or stage, reuse the Fox-Marth approach:
- an exit-criteria table like `docs/MATCHUP_COMPLETENESS.md`;
- `interaction_matrix.py --stage --characters`;
- explorer batches from its boundaries;
- directed witnesses via `search`.

## Status by area

**Characters** (`crates/ft-<name>`):
- 25 of 26 are registered, with all their specials; fail-closed branches
  are listed in Next, items 2 and 4.
- Family crates: fox, mars, mario, pikachu, captain, link.
- Two-fighter players share one model: Ice Climbers, and Sheik/Zelda (the
  sleeping form).
- Unregistered: Kirby. Its scenes fail at load.

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
- Ice Climbers: ice, Belay rope, Blizzard;
- Peach's rare pulls: Bob-omb, Beam Sword (with the swing states), Mr. Saturn;
- Yoshi's Story's Shy Guys.

Random items are unstarted.

**CPU** (`melee-cpu`): Nana's follow logic, Belay recovery, island
routing, getting up (behaviours 5, 6), tumbling (15) and the KO totals she
reads. CPU players
are out of scope.

**Slippi codes** (`melee_lib::slippi`, `ControllerFix`): UCF
0.73 beta, 0.74, 0.8 and 0.84; NeutralSpawn (2019, late-2019
entry-by-order and 2020 tables); the Stadium transformation preload and
Frozen Stadium; Dween; Frozen Stages; Widescreen 16:9. Not yet: the level
Fountain of Dreams platforms code (text not found), netplay codes.

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
| 2026-09-28 | Slippi replays run with the Slippi codes they were recorded with, ported from slippi-ssbm-asm and cited; codes a replay does not name are inferred only between discrete known versions, and frame zero checks the choice. |
| 2026-10-03 | Where a recording's date allows UCF 0.73 or 0.74, the runner runs both from the recorded inputs and the recorded facing at the first dashback they disagree on picks the version, which then runs the whole comparison from frame one (never a switch mid-run); the report prints it and `--controller-fix` overrides it (user, 2026-10-03: coordinator's call). |
| 2026-09-28 | Asynchronous disc reads that affect gameplay (Pokémon Stadium's forms) are external inputs, recorded from retail and replayed like pads; standalone runs use a documented default. |
