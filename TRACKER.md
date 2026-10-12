# Tracker

Legend: `[ ]` todo, `[~]` in progress (add date), `[x]` done, `[!]` blocked
(say on what). Keep this file short and current: it states what is true now
and what is next. Finished work lives in git history and `docs/PORT_NOTES/`;
do not keep a session log here.

## Current state (2026-10-10, fourth Slippi wave: Ice Climbers, Ness, effect order)

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
  - `~/melee-data/replays/public-v3.7-b` (502 games, sampled 2026-10-03;
    triage in its `_triage/` folder): **409 complete**, 85.8% of frames
    (247 when sampled); 58 diverge, 2 stop at an unported branch, 33 are
    refused at setup (unregistered Kirby, online games, doubles, CPU
    players, other stages, one setup-seed mismatch).
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
- Gates at the checkpoint (2026-10-10): release workspace 1780/0 (incl.
  `m4_gate` 318 and `m5_gate` 286), clippy clean, pytest 301, schema check
  clean. **The debug gate is red**: eight debug test binaries (`melee-lib`
  lib/allocation/api, `melee-sim` alloc_gate and m2-m5) overflow the 2 MB
  test-thread stack; they pass with `RUST_MIN_STACK` of 3 MB. The same
  overflow is on c39611fc, so it predates this wave (Backlog). The perf
  gate was not run at this checkpoint.
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

1. [~] **Slippi corpus.** Fixed 2026-10-10: the Ice Climbers explorer
   faults (one player's ice blocks pass each other, the block's efAsync at
   link 9, a shared stale table within one proc, behaviour 18's x221A_b3,
   the Ice spark on a Shy Guy, the grounded Blizzard row airborne, the ice
   block's shield bounce, Nana leaving the revival platform, her Belay
   hang's ECB lock, a sleeping Nana's single revival slot), PK Thunder's
   and PK Fire's generator order, `ftCo_8008EC90` (unarmoured captor), the
   wall-tech flash order and Link's hookshot removal. Remaining stops in
   `public-v3.7-b` (58 diverge; the checkpoint's batch is the list):
   - the hit-log victim keyed by GObj address (retail reuses a freed
     address; the port keys by unique id): a serial task, it can move every
     scenario;
   - Ice Climbers, about ten games: follower stops during Popo's Ice Shot
     (341/342), Nana sliding from a laser on Fountain of Dreams' moving
     platform, seed drift with Nana alone on a platform, `ftCo_800AC5A0`
     (steering with no knockback, unported); two one-ulp Nana position
     stops are not shown to be port faults (retail in Dolphin equals the
     port, `docs/SLIPPI.md`);
   - five games stop at EntryStart/Entry on Fountain of Dreams (the level
     side platforms code, text not found; a scratch experiment matches two);
   - known causes not yet fixed: Yoshi's plain GuardOn keeps stale
     powershield scratch (`slp_ps_fox_yoshi_t1300`, `docs/SLIPPI.md`);
     Game & Watch's seed drift at tick 5464 (`docs/PORT_NOTES/GAMEWATCH.md`);
   - explorer faults with recorded witnesses and no owner: "cliff camera
     scratch missing" (`procs.rs:252`), `ftCo_800DC920` (a thrown fighter's
     constraint release on death, `life.rs:705`), `ftCo_800B732C` (a target
     on a ledge above the CPU), "holding an item" in DamageIce / Catch /
     FlyReflectWall (not audited);
   - the 2019 Dream Land NeutralSpawn row has no Gecko text, so those games
     cannot be bridged; Stadium consoles that set per-player spawn points
     cannot be bridged until `--retail-inputs` carries the spawn point;
   - unported: `ftCo_8008EC90` with an armoured captor (x1828 = 3), the
     sliding Samus bomb, a special while holding an item (Dr. Mario's
     neutral special), a frozen fighter leaving the top, Mr. Saturn's idle
     and slide states;
   - the ECB lock is counted only inside collision callbacks: other states
     with an empty callback may have the gap fixed for Nana's Belay hang.
   Then a newer corpus: Slippi Online (per-frame RNG sync, netplay codes)
   and Gecko-list (3.3+) code detection instead of inference.
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
