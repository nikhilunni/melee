# Tracker

Legend: `[ ]` todo, `[~]` in progress (add date), `[x]` done, `[!]` blocked
(say on what). Keep this file short and current: it states what is true now
and what is next. Finished work lives in git history and `docs/PORT_NOTES/`;
do not keep a session log here.

## Current state (2026-10-11, sixth Slippi wave: display timing, console codes, runner speed)

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
  - `~/melee-data/replays/public-v3.7` (108 games): **107 complete**, 99.1%
    of frames;
  - `~/melee-data/replays/public-v3.7-b` (502 games, sampled 2026-10-03):
    **446 complete**, 90.9% of frames (247 when sampled); 23 diverge, 32
    are refused at setup (14 Slippi Online, 5 Kirby, doubles, CPU players,
    other stages), one setup-seed mismatch. The batch takes about two
    minutes on 10 cores (`docs/PERF.md` "Replay runner").
- **Console codes a replay does not record are read from its frames** by a
  dry pass that compares the candidate setups and then runs the whole
  comparison under one (never a switch mid-run; the report prints each
  choice; a flag overrides it): UCF 0.73/0.74/0.8/0.84 (bounded by the
  Slippi version, then the date, then the first dashback they disagree on),
  Dween (runs no fix: the recording holds its output), Frozen Stages
  (`--stage-codes`), Widescreen (`--screen`), the spawn rule (frame zero).
- **Recordings render after every tick** (`render_every_tick`, the default
  for bridges since 2026-10-11): older bridges ran two ticks per render,
  which is another game near a tether, the magnifier or a screen KO. A stop
  is "not a port fault" only when an every-tick retail recording equals the
  port there. Shown that way: a console's lag frame moves a screen KO's
  position (three games, recorded with `no_display_pass_before`); the
  tournament consoles of early 2020 ran the Widescreen code at 92:63 and,
  from 2020-03-04, level Fountain of Dreams platforms with entry by spawn
  order (both read from a replay's frames; code text constructed, not
  found).
- Gates at the checkpoint (2026-10-11): release and debug workspace
  1784/0 (`m4_gate` 318, `m5_gate` 287), clippy clean, pytest 316. The perf
  gate: see the wave's closing commit.
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

1. [~] **Slippi corpus.** 24 games still diverge (`target/checkpoint`
   batch of 2026-10-11; `docs/SLIPPI.md` has each wave's findings):
   - console lag frames, to be inferred by the runner under a strict rule
     (rerun the tick with no display pass; accept only if the whole frame
     then matches, seed included; report it as inferred): three screen KOs
     proven on retail, two more unproven (`DOC/Game_20191106T211939`,
     `PEACH/15_52_33`, bridgeable now with `neutral-spawn-2019`), two
     magnifier points one tick late (`FALCO/02_33_48`, `19_56_47 Sheik +
     Captain Falcon`), Samus on FoD (7813);
   - set aside with evidence: the pad-queue game (`20_31_01 Captain Falcon
     + Ice Climbers`), two one-ulp Nana positions;
   - open, undiagnosed: DK + Ganondorf (4779), Fox vs Dr. Mario PS (9684),
     Fox percent (1482), Ice Climbers + Samus PS (6698, Ice Shot), Link DL
     (3267, a bomb blast) and PS (10399), Mario vs Pikachu (6305), Mario vs
     Yoshi (6489), Marth vs Peach PS (5251), Falcon vs Pichu (2158), Fox vs
     Yoshi PS (1093), Yoshi dittos (9961), Falco vs Zelda FoD (8969);
   - known causes not yet fixed: Yoshi's plain GuardOn keeps stale
     powershield scratch (`slp_ps_fox_yoshi_t1300`); Game & Watch's seed
     drift (`docs/PORT_NOTES/GAMEWATCH.md`);
   - Stadium bridges leave the console's game at the first transformation
     (seen twice); undiagnosed;
   - item GObj identity differs from retail somewhere in 55 of 2239
     scenarios (`melee-sim gobj-diff`), untriaged;
   - explorer faults with recorded witnesses and no owner: "cliff camera
     scratch missing" (`procs.rs`), `ftCo_800DC920`, `ftCo_800B732C`,
     "holding an item" in DamageIce / Catch / FlyReflectWall;
   - unported: Mr. Saturn's idle and slide states (two recordings leave
     the port there at tick 3710), the sliding Samus bomb, `ftCo_8008EC90`
     with an armoured captor, Mario's held-item specials, a frozen fighter
     leaving the top, `ftCo_800AC5A0`.
   Then Slippi Online (14 games refused: per-frame RNG sync, netplay
   codes) and Gecko-list (3.3+) code detection instead of inference.
2. [ ] **Kirby**, last: copy abilities need every copied character's
   specials, all of which now exist.
3. [ ] **Fail-closed branches left from wave D**: Captain Falcon's sword
   Swing42, item-on-item push, Mr. Saturn's idle and slide states, UCF 0.84
   branches without witnesses (KO totals, Nana's behaviours 5, 6 and 15,
   DamageIce, the sword swings and Mr. Saturn knocked away are done).
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

**App** (`docs/APP.md`, 2026-10-04): Ghostty-style split. libmelee
(`melee-platform`: disc via `gc-disc`, file cache, catalog, menu flow,
session, wgpu renderer; C API) under native hosts: `apps/macos` (AppKit,
SwiftUI menus) and `crates/melee-web` (HTML/DOM menus, WebGPU canvas). The
only input is the Melee NTSC-U 1.02 disc image. All 25 characters and six
stages render; the wasm32 sim is bit-identical to native (492 corpus gates,
108 replays run inside wasm). Open:
- menus redesigned on both hosts (docs/DESIGN.md): disc art decoded at
  runtime (portraits, faces, stocks, emblems, name plates), stage previews
  rendered at runtime (1080p, ~1.5 s native, ~3.4 s web); nothing from the
  disc is committed. Open: web adopts DESIGN.md items 11-17; a rendered
  Sheik portrait (none on the disc); gamepads;
- render-to-texture (FoD reflection, Stadium screen feed), Link's bow,
  metal/low-poly, same-costume second-player tint, item poses per instance;
- wasm panics abort (no catch_unwind): the page offers the replay;
- renderer speed (2026-10-04): hardware sampling, unrolled and
  shape-specialised material pipelines, shared geometry buffers, no MSAA in
  the browser (Dawn: 112 -> 12.9 ms). Chrome full Retina: ~0.5 ms GPU,
  ~0.5 ms sim per tick; native 0.3-0.5 ms per frame (`render_frame --bench`);
- wasm float edge cases fixed (software fma, `gekko_math::cmp`, clippy
  bans std float math); `tools/wasm-check.sh` diffs match hashes.
## Backlog

- [ ] Profile `replay-batch`: a replay runs at about 2,000 frames/s per
      core (three `public-v3.7-b` games, 4 to 11 s each, 2026-10-11) against
      about 25,000 ticks/s for the headless sim (`docs/PERF.md`). Find where
      the factor of 12 goes (the dry passes that choose console codes, the
      per-frame comparison, loading) and cut it: the batch runs before and
      after every change.
- [ ] Untracked local scenarios (`harness/scenarios`, about 140 after this
      wave): most gate exact and are unregistered (explorer cases for DK,
      Link, Mewtwo, Yoshi, Falcon; 8 `samus_*`; camera and widescreen; 25
      replay start boundaries; 10 `slp_*` bridges). Run `particle-sites`
      and register them. Five `slp_*` bridges need `frames` cut to where
      retail's match ended. `x.toml` and `bridged.toml` are fixtures a
      pytest run leaked from `test_replay_bridge.py`.
- [ ] Perf gate: stripped `melee-sim` is 6,939,624 bytes, over the
      6,766,519 ceiling (user decides whether to raise `REVIEWED_SIZE`).
- [ ] `Match` is 96 MB and clone/new/reset cost 8-10 ms; a smaller state
      would make rollouts and tree search cheap.

- [ ] CI: `cargo gate`, clippy, harness pytest, `gen_schema.py --check`.
- [ ] 268 `unimplemented!` call sites (101 in `melee-ft`, 43 in `melee-cpu`;
      counted 2026-10-10 before this wave, comments excluded): each is a
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
| 2026-10-04 | Perf size floor raised to 6,549,232 stripped bytes for five new characters; each new character crate accepted at one duplicate label (the shared `CharacterTable` closure), as every older one is (user). |
| 2026-10-03 | Where a recording's date allows UCF 0.73 or 0.74, the runner runs both from the recorded inputs and the recorded facing at the first dashback they disagree on picks the version, which then runs the whole comparison from frame one (never a switch mid-run); the report prints it and `--controller-fix` overrides it (user, 2026-10-03: coordinator's call). |
| 2026-09-28 | Asynchronous disc reads that affect gameplay (Pokémon Stadium's forms) are external inputs, recorded from retail and replayed like pads; standalone runs use a documented default. |
