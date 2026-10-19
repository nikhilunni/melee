# Tracker

Legend: `[ ]` todo, `[~]` in progress (add date), `[x]` done, `[!]` blocked
(say on what). Keep this file short and current: it states what is true now
and what is next. Finished work lives in git history and `docs/PORT_NOTES/`;
do not keep a session log here.

## Current state (2026-10-18, after the tenth wave)

- **All 26 characters are registered and all six tournament stages run
  bit-exact.** Kirby (2026-10-12) has his movement, attacks, all four
  throws, Inhale (catch, mouthful, spit, swallow), Final Cutter, Hammer,
  Stone, the helmeted jumps and losing an ability; no copied special is
  ported (`docs/PORT_NOTES/KIRBY.md`, "Where this stands and how to
  resume"). Each character has a start boundary vs Fox on FD (`m4_gate`),
  a witness list in `m5_gate` and, except Kirby, explorer matches.
- **Slippi replays** (every tick's Pre Frame seed is checked, so a complete
  game has no random-stream drift):
  - `~/melee-data/replays/public-v3.7` (108 games): **108 complete**, every
    frame (one with an inferred lag frame);
  - `~/melee-data/replays/public-v3.7-b` (502 games, sampled 2026-10-03):
    **484 complete**, 96.7% of frames (247 complete when sampled): 458
    plain, 11 with one inferred lag frame, 15 Slippi Online; 5 stop, 13 are
    refused at setup (doubles, CPU players, other stages). Without lag
    frame inference 473 complete. The batch takes 2:15 on 10 cores.
  - **Slippi Online games run** (`replay_online.rs`, `docs/SLIPPI.md`): the
    seed is supplied before each tick as the netplay code writes it, codes
    come from the replay's Gecko list. Weaker there only: the end-of-tick
    seed is not compared, and the sign of a zero in `kb_vel` is free
    (Slippi Dolphin's JIT computes one `fnmsub` unfused; proven on retail).
  - **Lag frames are inferred** (`replay_lag_frames.rs`, `--no-lag-frames`
    turns it off): on a screen-KO position stop, a magnifier percent stop,
    a seed stop, or a Stadium seed stop at the end of a jumbotron mode
    entered without a draw, the runner retries with one display pass
    skipped and accepts only if the frame and a margin after it match,
    seeds included. All twelve inferred are proven on retail with a
    `no_display_pass_before` recording. A port fault that acts exactly
    like a skipped pass would pass the rule: count such games separately.
    An unconsumed controller poll (two games) is proven on retail and
    deliberately NOT inferred: nothing recorded pins where it falls.
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
- Gates at the wave-10 checkpoint (2026-10-18): release workspace
  1846/0 (324 test binaries), clippy clean, pytest 368, schema check
  clean, `m4_gate` and `m5_gate` green, replay batches as above with no
  game matching fewer frames (Kirby + Mr. Game & Watch moved 5100 to
  5388). **The perf gate still fails on size and one label, not on time**
  (19.6 ms per 600 ticks): stripped 7,079,088 bytes against the 6,876,693
  ceiling, and `ft-kirby`'s one duplicate label against a missing baseline
  of 0. Three `melee-ft` duplicate labels this wave added were removed
  (ae614c2d). Baselines are the user's call (`docs/PERF.md`).
- **`melee-sim gate --particles`** compares the whole particle system with
  the retail dump each tick (`docs/ORACLE.md`). Of the 112 recorded
  scenarios that passed the fighter and item gate but differed in a
  particle at the start of wave 10, **2 remain** (sweep 2026-10-18: 2866
  exact, 2 particle faults, 102 not compared, 14 base failures, all
  unregistered local recordings or Kirby's copy abilities): Pikachu's
  Quick Attack trail reading XRotN mid-tick (program 74,
  `corpus_v3_bf_yoshi_pikachu4_ec13743d5_p2`) and one Ice Climbers
  item-teardown generator (program 14000,
  `corpus_v3_dl_luigi_iceclimbers4_e86fddb60_p1`). Causes and fixes:
  `docs/PORT_NOTES/PARTICLE_GATE_2026_10.md`. Still opt-in (a user
  decision, below). Item GObj identity equals retail's in all but one
  unusable recording (`melee-sim gobj-diff`).
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

1. [~] **Slippi corpus.** 5 games stop in `public-v3.7-b`, each explained:
   - Kirby + Mr. Game & Watch: the hats are done and it now stops at 5388
     on a copied neutral special (`ftKb_Init_803C9DD0`, item 2);
   - two pad-queue games, proven on retail to be a console's unconsumed
     controller poll and not inferred (Falcon vs Pichu 2158, Falcon + Ice
     Climbers FoD 7022);
   - two one-ulp Nana positions (423, 200): every-tick retail equals the
     port, not the console; seeds, pads and the frame before are equal.
     Unexplained.
   Next for the oracle, in order of value:
   - **explorer**: both 2026-10 rounds' faults with retail witnesses are
     fixed and registered (`EXPLORER_CAMPAIGN_2026_10.md`, `_10B.md`;
     the second round ran every boundary without one, so every character
     now has matches off Final Destination). Explorer bridges render after
     every tick since 2026-10-18, so a bridged retail match follows the
     explorer's game. Next: more seeds across the roster (FoD, PS and DL
     are thin for most pairs);
   - **the particle gate's 2 remaining scenarios** (above), then wire the
     check into `m5_gate` and make it the default;
   - a newer or wider replay corpus (none is on disk beyond the two);
   - a display pass also writes drawn joint matrices that dynamics, the
     Thrown hip test and tether links read; the port models only part;
   - one Dream Land generator's x differs on 39 ticks of
     `slp_dl_peach_younglink_r155233_*` with equal seeds;
   - `ftCo_800AC5A0`'s stick comes from the fighter's heap address: the
     port has a table for 13 recorded setups, others fail closed;
   - unported and unreached: the captor that keeps its grab while in
     hitlag (`ftCo_8008EC90`, x1828 = 3), a frozen fighter leaving the top,
     Mr. Saturn's stage-spawned states, the inert item hitbox, Bob-omb's
     slide, a holding-an-item audit for DamageIce and FlyReflectWall,
     article removers that do not queue the generator-cursor walk
     (`docs/SLIPPI.md` audit; only Peach's parasol is witnessed);
   - ported from retail's table but unwitnessed (2026-10-18): an item hit
     by a Dark element (efSync 0x416) or a ReDead element (nothing
     spawned), `melee-it` `hurt.rs`;
   - a silent-divergence risk: `FighterCore::check_blast_zone` skips the
     check whenever the scratch is `MotionData::Life`, which stays set
     after a fighter leaves revival by a special (retail tests x2219_b1;
     the magnifier had the same fault, fixed in 2301bf13);
   - tooling: `bones-diff` compares local SRT only, so a world matrix off
     by ulps (the second leg's IK, 89c7cb48) passes it.
2. [~] **Kirby**: everything but the copy abilities. Done 2026-10-18: the
   five grafted hats, the Cutter beam on slopes, the aerial Final Cutter
   landing, the inhale's release and platform drop, and explorer rounds
   from all eight Kirby boundaries. Next, in the order
   `docs/PORT_NOTES/KIRBY.md` gives: copied specials one kind at a time
   (Fox and Falco first; `ftKb_Init_803C9DD0` stops the last Kirby replay
   and three explorer cases), inhaling items and pickup hand poses, the
   remaining fail-closed branches.
3. [ ] **Fail-closed branches left from wave D**: Captain Falcon's sword
   Swing42, item-on-item push, Mr. Saturn's idle and slide states, UCF 0.84
   branches without witnesses (KO totals, Nana's behaviours 5, 6 and 15,
   DamageIce, the sword swings and Mr. Saturn knocked away are done).
4. [~] **Particle positions in the gate.** 2 of 112 left (above); then
   make it the default in `m5_gate` (user decision).
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

- [x] `replay-batch` profiled and cut 2.7x (2026-10-11, `docs/PERF.md`
      "Replay runner"). A further 2.3x needs watch state in the sim; the
      commits are on branch `worktree-agent-abfdfd481090df427`, held out.
- [x] Untracked local scenarios: classified and registered 2026-10-12
      (`docs/PORT_NOTES/UNREGISTERED_SCENARIOS.md`).
- [ ] **User decisions waiting** (2026-10-12, still open 2026-10-18):
      perf baselines (size ceiling, `ft-kirby`'s label entry);
      whether to re-record the particle, bone and camera dumps of 35
      every-tick scenarios made at the wrong cadence, and what to do with
      about 20 GB of uncompressed particle dumps from passes that timed
      out (`find harness/traces -size +100M ! -name '*.zst'`);
      whether to model Slippi Dolphin's unfused `fnmsub` for netplay;
      whether lag-frame inference stays on by default and the particle
      gate stays opt-in; whether to push (main is far ahead of origin and
      the CI workflow has never run).
- [ ] `Match` is 96 MB and clone/new/reset cost 8-10 ms; a smaller state
      would make rollouts and tree search cheap.

- [x] CI workflow added 2026-10-11 (`.github/workflows/ci.yml`), not yet run.
- [ ] About 300 `unimplemented!` call sites (308 lines by a raw grep on
      2026-10-12, 273 on 2026-10-10: the growth is Kirby's fail-closed
      branches): each is a branch no gated scenario reaches; port them as
      explorer faults or new content reach them.
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
