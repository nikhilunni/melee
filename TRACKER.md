# Tracker

Legend: `[ ]` todo, `[~]` in progress (add date), `[x]` done, `[!]` blocked
(say on what). Keep this file short and current: it states what is true now
and what is next. Finished work lives in git history and `docs/PORT_NOTES/`;
do not keep a session log here.

## Current state (2026-10-24, wave 14 integrating)

- **Five of six handoff branches are on main:** grab/common edges, Kirby's
  Nayru's Love/Fire Breath/Ice Shot, match flow, random items, and Kongo
  Jungle/Jungle Japes. At `1dd22bd8`, the accepted sweep has **0 regressions**:
  3163 particle-exact, 112 not comparable, 209 existing local failures and
  one existing particle fault. Both replay corpora retain every baseline
  completion and matched frame: **108/108** and **486/502** complete.
  The root's full combat gate passes **293/293** and harness pytest **406**.
  CPU's 95 witnesses pass on its rebased branch, including the repaired
  persistent rapid-jab counter. Review found one remaining shared
  character-kind dispatch, now being moved into the character table; final
  verification of that adjustment, sweep and replay acceptance are
  pending a macOS executable-launch stall. It remains unmerged. Full wave
  checkpoint and fresh parity numbers follow CPU acceptance.

- **Wave 14 startup:** the release workspace checkpoint exposed a dormant
  invalid palette in retail's `EfKbSs.dat`; presentation now retains its
  decode error until a live particle selects that image, as retail defers
  the palette read until display. The strict archive reader still rejects
  it. All presentation tests pass. The N64 stage icon test now uses the
  disc's 48x48 dimensions for all three Past Stages. Release workspace
  checks completed (the corrected icon target rerun: 3/3), release clippy
  is clean, harness pytest 401, and both replay corpora retain 108/108 and
  484/502 complete. The fresh sweep has 0 regressions: 3075 particle-exact,
  102 not comparable, 307 existing local failures and one existing particle
  fault. These are startup measurements, before the integrations above.

- **Wave 13 chased breadth** (user, 2026-10-24: completeness is the only
  goal; close fail-closed sites and unported code before chasing single
  divergences). `docs/PARITY.md` and `harness/parity_inventory.py` measure
  it per retail function: at wave-14 startup, in-game logic was **42.6% ported or
  partial, 54.2% with inlined bodies and rows** (41.8 / 52.2 at the wave's
  start), with **235 fail-closed sites** (331). `harness/retail_coverage.py`
  (Dolphin Branch Watch, `docs/PORT_NOTES/RETAIL_COVERAGE.md`) measures
  what retail executes: 7,919 of 19,827 functions across 1,472 recorded
  scenarios; broader runs (items on, four players, other stages) add
  1,513 more, mostly items.
- **CPU is the remaining wave-13 handoff.** The original notes in
  `docs/handoffs/` describe the incoming branches; acceptance evidence for
  the five merged branches is under `target/wave14/`. Unsupported item,
  stage, stock-sharing and copied-special branches remain guarded.
  `docs/handoffs/WAVE13_FOLLOWUPS.md` is the original follow-up inventory;
  consult current code and acceptance results before assigning a fault.
- Gates on main at 03f801e2 (2026-10-24): sweep 3075 exact, 102 not
  comparable, 0 regressions against 26fee4c7 (55 fixed); the 307 failing
  and one particle fault are local scenarios not on main (the unmerged
  branches' recordings, `corpus_v3_ys_ganondorf_fox4_e281b1a7a_p1`).
  `public-v3.7` replays 108/108 complete; harness pytest 401. The debug
  workspace gate, `m5_gate` in full, wasm-check and the perf gate were not
  rerun this wave.
- **All 26 characters are registered and all six tournament stages run
  bit-exact; every Versus stage is named** (unported ones fail closed by
  name) and Yoshi's Island 64 and Kongo Jungle 64 are ported
  (`docs/PORT_NOTES/N64_STAGES.md`), plus Kongo Jungle and Jungle Japes
  with their unwitnessed hazard branches guarded. Kirby has his movement,
  attacks, throws, Inhale, Final Cutter, Hammer, Stone and 24 of the 25 copied
  neutral specials on main (Sheik's remains unstarted;
  `docs/PORT_NOTES/KIRBY.md`). Each character has a start boundary vs Fox
  on FD (`m4_gate`), a witness list in `m5_gate` and explorer matches.
- **Slippi replays** (every tick's Pre Frame seed is checked, so a complete
  game has no random-stream drift):
  - `~/melee-data/replays/public-v3.7` (108 games): **108 complete**, every
    frame (one with an inferred lag frame);
  - `~/melee-data/replays/public-v3.7-b` (502 games, sampled 2026-10-03):
    **484 complete**, 5,081,080 frames (2026-10-19) (247 complete when sampled): 458
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
- Gates at the wave-12 checkpoint (2026-10-19): release and debug
  workspace 1864/0, clippy clean, pytest 368, wasm32 = native (6003
  lines), `m4_gate` and `m5_gate` green, replay batches as above with no
  game matching fewer frames (Kirby + Mr. Game & Watch 5388 to 9909).
  Sweep (`melee-sim sweep`, every scenario with particles): 2954 exact,
  101 not comparable, 12 failing (all unregistered local recordings with
  data problems: missing traces, trace length, seed). **The perf gate
  fails on size and one label, not on time** (19.9 ms per 600 ticks; load
  58.5 ms, was 158): stripped 7,248,832 bytes against the 6,876,693
  ceiling, and `ft-kirby`'s one duplicate label (the `CharacterTable::new`
  closure every character crate has) against a missing baseline entry.
  Baselines are the user's call (`docs/PERF.md`).
- **`Match` is 22.5 MB (was 102.7) and clone takes 2.4 ms (was 7.8)** for
  FD Fox vs Marth (`cargo run --release -p melee-lib --example
  match_memory`): effect models share one callback buffer, only the banks
  a match can use are pooled (a Kirby match pools all), and a tree's
  orientation constraints are a list (21bd6fd4). No
  heap allocation after `Match::new` on any stage or character
  (`melee-lib/tests/allocation.rs`, 13 matches x 3600 ticks).
- **The particle check is on by default** in `melee-sim gate`, `sweep`
  and `m5_gate` (user decision 2026-10-19): every recorded scenario is
  particle-exact or says why its dump is not comparable
  (`docs/PORT_NOTES/PARTICLE_GATE_2026_10.md`). `bones-diff` compares world
  matrices where the port holds a joint clean and retail's dump does not
  list it dirty (`dirty_bones`, recorded since 2026-10-19). Item GObj
  identity equals retail's in all but one unusable recording
  (`melee-sim gobj-diff`).
- A bridge needs the replay's own boundary (`--game-start-seed`) wherever
  the stage draws at creation: Final Destination, Fountain of Dreams,
  a transforming Pokemon Stadium, Dream Land past tick 3000, and any Ice
  Climbers game. The recorder's particle, bone and camera passes ran at
  the wrong cadence for every-tick scenarios until 2026-10-12: existing
  dumps of such scenarios are not re-recorded (`docs/SLIPPI.md`).
- Tooling (`AGENTS.md` "Slippi replays", `docs/SLIPPI.md`):
  - `make_boundary.py`: any stage, characters, costumes; with
    `--no-register`, ports, Gecko codes, spawn rule, timer, and
    `--game-start-seed` (a per-replay boundary, needed on FD and FoD);
  - `record_many.py` with Gecko codes (`MELEE_GECKO_DIR=~/melee-data/gecko`);
    a scenario's `after_map = true` samples retail at the end of each
    fighter's map proc (Slippi before 3.4.0 samples there);
  - `melee-sim replay`, `replay-batch` (`pN.input_seed` = RNG drift on the
    tick before), and the replay-to-retail bridge (`--retail-inputs`,
    `harness/slippi_to_scenario.py`), which follows a replay on every stage;
  - `melee-sim sweep` (all scenarios on every core, `--baseline`,
    `--filter`, `--lock`) and `sweep-compare`; the coordinator keeps the
    baseline of the commit agents start from in `target/sweeps/`;
  - `MELEE_DATA_ROOT` for worktrees; `docs/AGENT_BRIEF.md`;
    `tools/agent-merge/pick.sh` (array lengths in `m5_gate` lists often
    need fixing after a merge: compile the tests).

## Next (recommended order)

1. [~] **Merge the six wave-13 handoff branches** (`docs/handoffs/*.md`):
   rebase each onto main, drop or re-guard its WIP commits, then a full
   sweep and `replay-batch` per branch. Suggested order: grab/common edges,
   Kirby copies, match flow, random items, Kongo/Japes (reconcile its
   barrel with main's `barrel.rs`), CPU players (largest; conflicts with
   the all-stages `make_boundary.py`). The first five are merged; CPU's
   final sweep/replay proof and the combined wave checkpoint remain.
2. [ ] **Breadth wave 14**, from `docs/PARITY.md` (rerun
   `parity_inventory.py` after the merges) and `WAVE13_FOLLOWUPS.md`: the
   item-use fighter rows (HeavyGet, Lift, swing and shooting items), more
   random item kinds, the 19 Versus stages still unported, Sheik's copy,
   CPU stage awareness, stamina/coin/handicap/time modes, then the rest of
   the fail-closed backlog. Record on a quiet machine: under heavy load
   about 8% of retail recordings differed between runs, with or without
   the coverage build (`RETAIL_COVERAGE.md`).
3. [~] **Slippi corpus.** 5 games stop in `public-v3.7-b`, each explained:
   - Kirby + Mr. Game & Watch: the copied Chef is done and it now stops at
     9909: Mr. Game & Watch's x is ~0.008 off in his aerial Fire (374) at
     FoD's left edge beside Kirby; not yet diagnosed;
   - two pad-queue games, proven on retail to be a console's unconsumed
     controller poll and not inferred (Falcon vs Pichu 2158, Falcon + Ice
     Climbers FoD 7022);
   - two one-ulp Nana positions (423, 200): every-tick retail equals the
     port, not the console; seeds, pads and the frame before are equal.
     Unexplained.
   Next for the oracle, in order of value:
   - **explorer**: three 2026-10 rounds, every fault with a retail witness
     fixed and registered (`EXPLORER_CAMPAIGN_2026_10.md`, `_10B.md`,
     `_10C.md`; 10C added FoD, PS and DL boundaries for the six characters
     without one). Next: more seeds, and pairs still thin off FD;
   - a newer or wider replay corpus (none is on disk beyond the two);
   - a display pass also writes drawn joint matrices that dynamics, the
     Thrown hip test and tether links read; the port models only part;
   - one Dream Land generator's x differs on 39 ticks of
     `slp_dl_peach_younglink_r155233_*` with equal seeds;
   - `ftCo_800AC5A0`'s stick comes from the fighter's heap address: the
     port has a table for 13 recorded setups, others fail closed;
   - unported and unreached: the captor that keeps its grab while in
     hitlag (`ftCo_8008EC90`, x1828 = 3), a frozen fighter leaving the top,
     Mr. Saturn's stage-spawned states 7-10 (need a stage- or
     random-spawned item), Bob-omb's slide, a holding-an-item audit for
     DamageIce and FlyReflectWall, article removers that do not queue the
     generator-cursor walk (`docs/SLIPPI.md` audit), CatchPull losing its
     floor (same code as CatchWait, unwitnessed);
   - ported from retail's table but unwitnessed: a ReDead element
     (`melee-it` `hurt.rs`; no loaded script carries one).
4. [~] **Kirby's copied specials**: 24 of 25 kinds on main; Sheik's Needle Storm is
   unstarted (its boundary exists; `charge_flash.rs`'s Sheik arm is empty).
   Then inhaling items and pickup hand poses (`KIRBY.md` item 3).
5. [ ] **Match clone below 1 ms**: the floor is the 64 ready slots per
   effect model (~12 of 19 MB); shared slots need buffer-reusing copies in
   hsd-anim, or per-model limits proven from retail. Fighter track
   buffers are another 4 MB.

For a new matchup or stage, reuse the Fox-Marth approach:
- an exit-criteria table like `docs/MATCHUP_COMPLETENESS.md`;
- `interaction_matrix.py --stage --characters`;
- explorer batches from its boundaries;
- directed witnesses via `search`.

## Status by area

**Historical parity inventory** (`docs/PARITY.md`,
`harness/parity_inventory.py`, before the handoff integrations; rerun at
wave close): by retail code bytes, in-game logic is 41.8% ported or partial
(52.2% counting callees ported inline and implemented motion rows); all of
retail 27.6% (34.8%). 331 fail-closed sites, clustered into 32 work items.

**Characters** (`crates/ft-<name>`):
- All 26 are registered; 25 have all their specials (Kirby: `Next` 2); fail-closed branches
  are listed in Next, items 2 and 4.
- Family crates: fox, mars, mario, pikachu, captain, link, purin, gamewatch.
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

Random-item spawning and Capsule/Box/Barrel are on main, witnessed for
two-player FD/Battlefield setups. Party Ball, other random item kinds,
heavy-item fighter rows and unwitnessed container blasts remain guarded.

**CPU** (`melee-cpu`): Nana's follow logic, Belay recovery, island
routing, getting up (behaviours 5, 6), tumbling (15) and the KO totals she
reads. CPU players
are out of scope.

**Slippi codes** (`melee_lib::slippi`, `ControllerFix`): UCF
0.73 beta, 0.74, 0.8 and 0.84; NeutralSpawn (2019, late-2019
entry-by-order and 2020 tables); the Stadium transformation preload and
Frozen Stadium; Dween; Frozen Stages; Widescreen 16:9. Not yet: the level
Fountain of Dreams platforms code (text not found), netplay codes.

**Completeness backlog**: the in-game Start pause, retail menus, results and
single-player modes. These remain in scope; breadth work precedes individual
replay fixes (user, 2026-10-24).

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
- [ ] **User decisions waiting** (2026-10-12, still open 2026-10-19):
      perf baselines (size ceiling; `ft-kirby`'s label entry, recommended
      2026-10-19); whether to re-record the particle, bone and camera
      dumps of 35 every-tick scenarios made at the wrong cadence; whether
      to model Slippi Dolphin's unfused `fnmsub` for netplay; whether
      lag-frame inference stays on by default; whether to push (main is
      far ahead of origin and the CI workflow has never run). Decided
      2026-10-19: the particle check is the default; unreferenced orphan
      traces deleted and large uncompressed traces compressed.

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
| 2026-09-09 | Motion-state tables, static character tables, family crates, concrete core and shell, no per-tick allocation (architecture rules in `AGENTS.md`). |
| 2026-09-26 | Record with headless Dolphin by default; windowed only for human play and menu driving. |
| 2026-09-26 | The explorer corpus starts from registered retail boundaries, so every case is replayable in Dolphin. |
| 2026-10-24 | Subagents on any model, run widely in parallel (supersedes the 2026-09-28 Opus-only rule). `AGENTS.md` is the single instructions file for every agent. |
| 2026-09-28 | Perf size baseline raised to 6,444,304 stripped bytes after wave C (user). |
| 2026-09-28 | Slippi replays run with the Slippi codes they were recorded with, ported from slippi-ssbm-asm and cited; codes a replay does not name are inferred only between discrete known versions, and frame zero checks the choice. |
| 2026-10-04 | Perf size floor raised to 6,549,232 stripped bytes for five new characters; each new character crate accepted at one duplicate label (the shared `CharacterTable` closure), as every older one is (user). |
| 2026-10-03 | Where a recording's date allows UCF 0.73 or 0.74, the runner runs both from the recorded inputs and the recorded facing at the first dashback they disagree on picks the version, which then runs the whole comparison from frame one (never a switch mid-run); the report prints it and `--controller-fix` overrides it (user, 2026-10-03: coordinator's call). |
| 2026-09-28 | Asynchronous disc reads that affect gameplay (Pokémon Stadium's forms) are external inputs, recorded from retail and replayed like pads; standalone runs use a documented default. |
