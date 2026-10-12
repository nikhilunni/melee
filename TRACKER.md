# Tracker

Legend: `[ ]` todo, `[~]` in progress (add date), `[x]` done, `[!]` blocked
(say on what). Keep this file short and current: it states what is true now
and what is next. Finished work lives in git history and `docs/PORT_NOTES/`;
do not keep a session log here.

## Current state (2026-10-12, seventh Slippi wave: lag frames, Kirby's first stage, Stadium bridges)

- **All 26 characters are registered and all six tournament stages run
  bit-exact.** Kirby (2026-10-12) has his movement, normals, aerials,
  throws taken and given (down throw), and the Inhale that catches
  nothing; his other specials, three throws and every copy ability are
  `unimplemented!` by name (`docs/PORT_NOTES/KIRBY.md`). Each character
  has a start boundary vs Fox on FD (`m4_gate`), a witness list in
  `m5_gate` and, except Kirby, explorer matches.
- **Slippi replays** (every tick's Pre Frame seed is checked, so a complete
  game has no random-stream drift):
  - `~/melee-data/replays/public-v3.7` (108 games): **108 complete**, every
    frame (one with an inferred lag frame);
  - `~/melee-data/replays/public-v3.7-b` (502 games, sampled 2026-10-03):
    **462 complete** (8 of them with one inferred lag frame), 92.5% of
    frames (247 complete when sampled); 13 stop (5 at Kirby's unported
    moves), 27 are refused at setup (14 Slippi Online, doubles, CPU
    players, other stages). The batch takes about two minutes on 10 cores.
  - **Lag frames are inferred** (`replay_lag_frames.rs`, `--no-lag-frames`
    turns it off): on a screen-KO position stop, a magnifier percent stop
    or a seed stop the runner retries with one display pass skipped and
    accepts only if the frame and a margin after it match, seeds included.
    Seven of the nine inferred are proven on retail with a
    `no_display_pass_before` recording; `DOC/Game_20191106T211939` and
    `PEACH/15_52_33` are not. A port fault that acts exactly like a
    skipped pass would pass the rule: count such games separately.
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
- Gates at the checkpoint (2026-10-12): release workspace 1804/0
  (`m4_gate` 321, `m5_gate` 289), clippy clean. The perf gate fails on
  `ft-kirby`'s duplicate label (1 against a missing baseline of 0; every
  character crate has that one) and, measured under load, on time; the
  stripped size is 6,769,600 of a 6,876,693 ceiling. Baselines are the
  user's call (`docs/PERF.md`).
- A bridge needs the replay's own boundary (`--game-start-seed`) wherever
  the stage draws at creation: Final Destination, Fountain of Dreams,
  a transforming Pokemon Stadium, Dream Land past tick 3000, and any Ice
  Climbers game. The recorder's particle, bone and camera passes ran at
  the wrong cadence for every-tick scenarios until 2026-10-12: existing
  dumps of such scenarios are not re-recorded (`docs/SLIPPI.md`).
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

1. [~] **Slippi corpus.** 13 games stop (`target/checkpoint` batch of
   2026-10-12):
   - five at Kirby's unported moves (item 2);
   - open: Fox vs Dr. Mario PS (9684: every-tick retail equals the port,
     the console differs where the jumbotron changes mode), Marth vs Peach
     PS (5251: a thrown Mr. Saturn's bounce at 3709, `it-dosei`), Peach vs
     Young Link DL (11591, Peach hit out of float);
   - every-tick retail equals the port, the console differs (pad queue):
     Falcon vs Pichu (2158), Falcon + Ice Climbers FoD (7022); two one-ulp
     Nana positions (423 and 200; the second re-recorded every tick, the
     first not);
   - the bridge sometimes feeds retail pads that are not the console's
     (Peach vs Young Link from 3196, a Squall Hammer game near 350);
   - one Roy game needs setup; 14 are Slippi Online;
   - particle order with equal seeds: Yoshi's aerial Egg Throw (four ticks
     of `slp_fod_yoshi_yoshi_r1_t10100`), one Dream Land generator's x in
     `slp_dl_donkeykong_ganondorf_gs_r1_t4900`;
   - a display pass also writes drawn joint matrices that dynamics, the
     Thrown hip test and tether links read; the port models only part of
     that (no divergence known);
   - `slp_bf_luigi_iceclimbers_t2300`: Luigi's aerial Cyclone reads stale
     heap at Fighter +0x222C; the cold scene must carry the boundary's word;
   - item GObj identity differs from retail somewhere in 55 of 2239
     scenarios (`melee-sim gobj-diff`), untriaged;
   - explorer faults with witnesses and no owner: "cliff camera scratch
     missing" (`procs.rs`), `ftCo_800DC920`, `ftCo_800B732C`, "holding an
     item" in DamageIce / Catch / FlyReflectWall;
   - unported: Mr. Saturn's idle and slide states, the sliding Samus bomb
     and the shared item slide, `ftCo_8008EC90` with an armoured captor,
     Mario's held-item specials, a frozen fighter leaving the top,
     `ftCo_800AC5A0`, other Game & Watch article removers' cursor walk.
2. [~] **Kirby** (2026-10-12): stage 1 landed. Next: Final Cutter, Inhale
   catching a fighter (swallow, spit, copy; CaptureKirby on the victim),
   the forward, back and up throws, Hammer, Stone; then the copy abilities
   one kind at a time, the helmeted jumps, losing an ability. Remove the
   Kirby exemption in `melee-lib/tests/presentation_characters.rs` when
   his specials exist.
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
- All 26 are registered; 25 have all their specials (Kirby: `Next` 2); fail-closed branches
  are listed in Next, items 2 and 4.
- Family crates: fox, mars, mario, pikachu, captain, link.
- Two-fighter players share one model: Ice Climbers, and Sheik/Zelda (the
  sleeping form).

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
- [x] Untracked local scenarios: classified and registered 2026-10-12
      (`docs/PORT_NOTES/UNREGISTERED_SCENARIOS.md`).
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
