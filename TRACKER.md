# Tracker

Legend: `[ ]` todo, `[~]` in progress (add date), `[x]` done, `[!]` blocked
(say on what). Keep this file short and current: it states what is true now
and what is next. Finished work lives in git history and `docs/PORT_NOTES/`;
do not keep a session log here.

## Current state (2026-10-03, Slippi wave)

- **20 of 26 characters and all six tournament stages run bit-exact**, with
  every special. Registered: Fox, Falco, Marth, Roy, Captain Falcon,
  Ganondorf, Peach, Yoshi, Jigglypuff, Pikachu, Pichu, Mario, Dr. Mario,
  Luigi, Samus, Sheik, Zelda, Ice Climbers, Link, Young Link.
- **Slippi replays.** On the 108 sampled 2019-2020 tournament games
  (`~/melee-data/replays/public-v3.7`), 51 run by default (the rest predate
  UCF 0.74) and **44 match to the last frame** (22 before the wave). With
  `--controller-fix ucf-0.74` forced on the 57 older games, **85 of 108**
  match to the last frame (84.8% of all frames).
- The wave fixed seventeen faults, each with a retail witness unless noted in
  its commit: stale-move entries (repeated down tilt, item hits mid-move),
  Sheik's needle cancel on the shield bit, Slippi's Stadium preload and
  Frozen Stadium codes, the slash/slash clank sound, angled forward smash
  rows, Jigglypuff's multijump platform landing, the late-2019 NeutralSpawn
  entry delay, the third jab's rapid-jab check, electric phantom hitlag,
  grabs blocked by a wall, a shield hit while intangible, the Phantasm
  ghost's hitbox size, Sleep before respawn, Pikachu's Quick Attack spark
  flush, and the item clank rehit timer.
- Gates at the wave checkpoint: release workspace 1680/0 (incl. `m5_gate`,
  275 tests), clippy clean, pytest 280, schema check clean. Perf gate
  passes: 6.12 MB stripped, 19.6 ms per 600 ticks, 162 ms load.
- Tooling:
  - `make_boundary.py` creates any stage, character and costume boundary,
    and (with `--no-register`) ports, Gecko codes, spawn rule and timer;
  - `explore_batch.py --boundary` runs the explorer from it;
  - `record_many.py` records in parallel, with Gecko codes (`gecko = [..]`,
    `MELEE_GECKO_DIR`; the local code text is in `~/melee-data/gecko`);
  - `melee-sim search`, `triage`, `replay`, `replay-batch`
    (`--controller-fix` names the UCF version; a `pN.input_seed` stop is
    RNG drift on the tick before);
  - **replay-to-retail bridge**: `melee-sim replay <slp> --retail-inputs`
    plus `harness/slippi_to_scenario.py` play a replay's inputs, ports,
    codes, timer and seed on retail, so the port gates against the full
    retail trace (`SLIPPI_REPLAY_WITNESSES`; `docs/SLIPPI.md`);
  - `MELEE_DATA_ROOT` lets worktrees use the main checkout's data, for
    replays and scenarios;
  - `docs/AGENT_BRIEF.md` briefs worktree agents;
  - `tools/agent-merge/pick.sh` merges their commits (compile-check the
    tests after each merge: the list resolver can drop a brace).

## Next (recommended order)

1. [~] **Slippi replay corpus (milestone 7), 2026-10-03.**
   - **RNG drift first.** Three of the seven default-run stops are silent
     RNG drift that surfaces hundreds of ticks later: `Peach vs Fox [DL]`
     (drift in ticks 4008-4999, retail draws about 60-90 more),
     `Marth + Fox (FoD)` (two draws ahead at 6556, during Fox's up throw
     after a shine: a periodic spark flushed at the wrong point is the
     suspect), `Marth + Fox (PS)` (Marth's idle re-roll at 4167; the
     jumbotron camera test is the suspect). Compare the port's seed with
     Slippi's pre-frame `random_seed` in the replay runner to find the
     first drifting tick, then bridge that window to retail.
   - Other default-run stops: `Falco + Sheik (BF)` 6003 (DamageFlyLw vs
     FlyRoll, likely drift too); `Fox + Sheik (FoD)` 8405 (Sheik lands at a
     different height); `Sheik + Falco (DL)` 1973 (Sheik x off by 0.2 in
     Fall, wind-sized); `Marth + Peach (BF)` 7087 (Peach's facing in Turn;
     probably a UCF version matter, the witness was recorded with 0.8).
   - **UCF 0.73 beta.** Port its dashback program so the 57 early-2019
     games run by default; forced to 0.74, 41 of them already complete.
     Then triage the 16 that stop (motion, position and three
     facing-direction stops).
   - Same digital-only shoulder test as Sheik's needles, unfixed: Samus's
     Charge Shot cancel (`ft-samus/src/special_n.rs`, retail 0x80129BD8);
     audit the other `DIGITAL_SHOULDERS` uses (shield, grab escape, fall,
     life, Yoshi's shield) against the asm.
   - Same rule as the item clank fix, unfixed for lack of a witness:
     item-on-item clank (`it_8026FE68`). The effect seal between two
     motion changes in one tick is fixed only for Pikachu's zip.
   - Unverified in `ftColl_80078A2C`: the per-capsule grabbable flag and
     the `lbColl_8000ACFC` victim-list test.
   - Then: a larger and newer corpus (Slippi Online needs the per-frame RNG
     sync, netplay codes such as FreezeDeadUpFallPhysics and
     PreventWobbling), and Gecko-list (3.3+) code detection instead of
     inference.
2. [ ] **Remaining fail-closed branches from wave D**: KO totals once
   another player has fallen (needs `dmg.x18c4_source_ply`); Nana's CPU
   behaviour 5; strong Ice hits (DamageIce); sword dash-swing friction,
   Swing42 and hit mid-swing; Mr. Saturn idle/slide/knocked; item-on-item
   push; UCF 0.84 branches without witnesses.
3. [ ] **The remaining six characters**, in rough order of difficulty:
   Bowser; Donkey Kong (cargo carry); Mr. Game & Watch; Ness (PK Thunder,
   yo-yo); Mewtwo; Kirby (copy abilities need every copied character's
   specials). Each goes through the bring-up recipe in `docs/AGENT_BRIEF.md`.
4. [ ] **Particle positions in the gate.** The gate checks particle RNG
   order and generators, not positions (wave D found one class of bug this
   way: sound draws out of script order). Add a per-tick particle digest to
   the gate, then fix what it finds.
5. [ ] **Tick-path allocations.** Dream Land re-decodes its background
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
- 20 of 26 are registered, with all their specials; fail-closed branches
  are listed in Next, item 2.
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
- Ice Climbers: ice, Belay rope, Blizzard;
- Peach's rare pulls: Bob-omb, Beam Sword (with the swing states), Mr. Saturn;
- Yoshi's Story's Shy Guys.

Random items are unstarted.

**CPU** (`melee-cpu`): Nana's follow logic and Belay recovery. CPU players
are out of scope.

**Slippi codes** (`melee_lib::slippi`, `ControllerFix`): UCF 0.74, 0.8 and
0.84; NeutralSpawn (2019, late-2019 entry-by-order and 2020 tables); the
Stadium transformation preload and Frozen Stadium. Not yet: Dween, UCF 0.73,
netplay codes.

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
| 2026-09-28 | Asynchronous disc reads that affect gameplay (Pokémon Stadium's forms) are external inputs, recorded from retail and replayed like pads; standalone runs use a documented default. |
