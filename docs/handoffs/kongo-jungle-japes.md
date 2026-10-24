> **Branch** `worktree-agent-a8cac8906008453e4` (tip `9c82dd10`), not merged into main. Based on 26fee4c7. No full sweep or replay-batch has run (both needed: 38f6446b changes Dead-state physics on every stage, 40391e00 particle-bank choice). **7cee9f91 is WIP** (Klaptrap path rider, never called or gated). To merge, rebase onto main and reconcile with main's barrel.rs / BarrelWait and Stage names as described below, then sweep and replay-batch.

# Handoff: Kongo Jungle and Jungle Japes

Branch `worktree-agent-a8cac8906008453e4` (worktree
`.claude/worktrees/agent-a8cac8906008453e4`), rebased onto main `26fee4c7`
before the last two commits were added. It has not been rebased onto anything
later and has not been swept.

## Task and scope

Port Kongo Jungle (`grkongo.c`, GrKg.dat, StKind 0x05, `GrKind::Kongo`) and
Jungle Japes (`grgarden.c`, GrGd.dat, StKind 0x0C, `GrKind::Garden`) in full:
stage data and procs, the moving logs and the barrel cannon, the river current
and the Klaptrap, every RNG draw in retail order, and the stage items and
enemies. Witnesses: a start boundary against Fox on each stage, explorer
matches, directed witnesses (fighter in a barrel, in the river, bitten by a
Klaptrap), all registered in `m5_gate`. The CPU's awareness of Kongo barrels
(melee-cpu) was left to the CPU agent.

## Commits (oldest first)

| Commit | Status | Contents |
|---|---|---|
| `d16aed36` | verified | hsd spline: Bezier, B-spline and cardinal points, arc-length bisection (`hsd-archive` `desc/spline.rs`, `hsd-anim` `spline.rs`) |
| `40391e00` | verified | Jungle Japes: `melee-gr/src/garden/`, `it-yaku` (stage material item kind 0xA0, the Klaptrap bite), river through `Wind::water` and `FighterPhysics.water_splash`, `scene_stage/garden.rs`, `initial_state/stage_garden.rs`, `Stage::JungleJapes`, boundary `start_jj_fox_marth4` |
| `3d8c65b2` | verified | Kongo Jungle base: `melee-gr/src/kongo/{mod,planks,barrel,events,procs,tests}.rs`, the bridge and its planks, platform ends, barrel spin; melee-mp joint event recording; live spawn markers; `Stage::KongoJungle`, boundary `start_kj_fox_marth4` |
| `38f6446b` | verified | DeadDown/Left/Right physics runs free-flight physics (the procUpdate tail: the river carries dead bodies); m5 +2 JJ matches |
| `32059330` | verified on its witnesses, then guarded (see next) | Kongo barrel catch, BarrelWait, launch (`melee-ft/src/fighter/barrel.rs`, row 293), logs (maps 1, 2) and passers-by (maps 7, 8, 9), event models preloaded and recreated |
| `8bb668dc` | verified | A catch by a moving barrel, or a fighter held by one, fails closed; m5 +5 KJ matches (719 -> 724) |
| `43efbadb` | docs | `docs/COVERAGE_AUDIT.md` rows for the open and unreachable Kongo sites |
| `7cee9f91` | **WIP** | Klaptrap path rider (`melee-gr/src/kongo/path.rs`: grKongo_801D651C / 801D6668, Ground_801C4B50), `hsd_anim::spline::spline_tangent` (lbShadow_8000E9F0, linear and B-spline only), `ItemPose::rewound` (lb_8000B134 per sampled step). Compiles, has no caller, never gated |

## Verified witnesses (all exact, particles included)

- `start_jj_fox_marth4` and `_cold`, `start_kj_fox_marth4` and `_cold`: 600
  ticks, 62 keys; `m4_gate` tests `jungle_japes_and_kongo_jungle_start_particle_rng_order`
  and `start_jj_and_kj_fox_marth4_600`.
- In `CORPUS_V3_MATCHES`: `corpus_v3_jj_fox_marth4_ee133b82f_p0` (1367) and
  `_ee98155f3_p0` (1791): Klaptrap 30-damage bites, river entries and KOs.
  `corpus_v3_kj_fox_marth4_e0fcbde24_p0` (2236), `_e11788094_p2` (3240),
  `_e726cfdde_p1`, `_e9943b4ab_p0`, `_ec13743d5_p1` (1666 each): the bridge,
  the barrel's spin and path, the first event (tick 1664) with its logs and
  passers-by.
- Exact but not registered (they stop 2 ticks after the event):
  `corpus_v3_kj_fox_marth4_e00f31913_p1`, `_e6af4a7bb_p2`, `_e726cfdde_p2`,
  `_e89a89d0e_p1`/`_p2`, `_ef89b3e70_p1`/`_p2` (these now fail closed at a
  barrel), and `corpus_v3_kj_fox_marth4_e1611c835_p0`/`_p1` (barrel catch and
  launch, exact before the guard; the barrel was moving, so they stop now).
- The four other JJ explorer cases (`corpus_v3_jj_fox_marth4_e573e2d95_p0`,
  `_e0fcf0c70_p0`, `_eb17a6598_p0`, `_ef89b3e70_p0`) match on every key but
  their particle dumps are partial (dump timeouts), so they are not registered.

## Where it stopped

1. **Barrel position timing (open, frequent).** Explorer round
   `target/kj/explore_kj2` (60 seeds after 60 from `start_kj_fox_marth4`)
   showed the port's copy of the barrel's position (stage_info x730, written by
   Ground_801C4D70 from grKongo_801D5774's `lb_8000B1CC(jobj)`; the port's
   `stage.barrel_position` in `scene_stage/kongo.rs`, BARREL arm) a step off
   retail's while the barrel moves along its path. A held Fox sits 1.48 units
   behind (`corpus_v3_kj_fox_marth4_e0c8363c7_p1`, tick 1032); catches land a
   tick early or late (`_ef5e3e043_p0` 158, `_e255c070a_p1` 706,
   `_e8be4d273_p2` 1059, `_e7254fba4_p0` 1611, `_e41b2c2b8_p1` 2014). Ruled
   out: the distance arithmetic (fn_801D8134 is separate fmuls, dz^2 + (dx^2 +
   dy^2), same as the port); x1A6A (no Damage state sets it); bury_timer_2's
   reset at respawn (ftCo_800C09B4, fighter.c:522: true in retail, the port
   lacks it, but adding it changed none of these; it was reverted as
   unwitnessed). Likely cause: the barrel model's path animation is advanced at
   a different point than retail's relative to the barrel proc, or
   `joint_matrix` reads a fresher or staler parent matrix than
   HSD_JObjSetupMatrix does. The guard (`barrel_moved`, `barrel_near`) stops
   any eligible fighter within twice the catch radius of a moving barrel.
2. **Klaptrap event (open, frequent):** `grKongo_801D7BBC` Klaptrap is
   `unimplemented!`; 77 of 180 kj2 cases stop there. WIP `7cee9f91` holds the
   rider's math.
3. Rare: `grKongo_801D5340` a second map-1 log while the first lives (1 case,
   frame 3633), `ftCoD5A30` revival platform reaches floor (1 case), both
   fail closed, neither bridged.

## Next steps

1. Fix the barrel timing: compare the retail order in grKongo_801D5774 (barrel
   proc) against the port's stage anim tick in `melee-lib/src/frame.rs` and
   the BARREL arm of `scene_stage/kongo.rs`; check whether retail's
   `lb_8000B1CC` sees the model after or before this tick's
   `HSD_JObjAnimAll`. `record.py --bones N` on `e0c8363c7_p1` gives the barrel
   joint's retail matrix. Then drop the guard and gate all kj scenarios above.
2. Port the Klaptrap (itklap.c) on the WIP rider: states [0,1,2,3,2]; spawn
   it_802E1820 (zako init, facing 0, state 0), pushable false; phys 0/1 toggle
   when `lb_8000B134` (use `ItemPose::rewound`) and `Randi(100) < 30`; coll
   0/1 follows the rider (y - 5.0 in double) and its rotation, falls (state 2,
   blast zone on, position = child bone world position) when the child's z >=
   0 or the rider is gone; state 2 turns rotX toward pi/2 by 0.034906585;
   state 4 spins on damage ((Randf - 0.5) * 0.17453292); state 3 is the barrel
   (on_accessory it_802E215C, launch it_802E2330, catch radius check
   grKongo_801D8078); destroying it destroys the rider. The scene creates map
   11 (empty JObj) with `Rider::new`, then the Klaptrap at the rider (if that
   fails, remove the rider). The kind is GrKg itemdata 0xDA.
3. Register the scenarios that then gate exact (listed above) in `m5_gate`,
   rerun `explore_batch.py` on `start_kj_fox_marth4` for longer barrel and log
   coverage, and write directed witnesses (fighter in the barrel through a
   launch; landing on a log).
4. Full sweep against main's baseline and `replay-batch` over
   `~/melee-data/replays/public-v3.7`: `38f6446b` changes Dead-state physics
   for every stage, and `40391e00` touches particle bank selection, so both
   reach replays. Neither has been swept or replayed after the rebase (the
   sweep was cancelled).

## Reconciling with main (after 26fee4c7)

- Main now has Yoshi's Island 64 and Kongo Jungle 64, with its own
  `melee-ft/src/fighter/barrel.rs` (Parameters from PlCo +4DC/+4E4/+5E0,
  `BarrelShot`, LEDGE_OPTION exclusion, the BarrelWait row). This branch's
  `barrel.rs` (from `32059330`: `enter_barrel_wait`, `launch_from_barrel`,
  `Status.barrel_cooldown`, `barrel_fire_requested`, `assets.barrel_cooldown`)
  covers the same retail functions. Keep main's file and its names; port this
  branch's scene side (`barrel_catch`, `barrel_fire_request`,
  `barrel_accessory`, `step_hold` in `melee-lib/src/scene_stage/kongo.rs`) to
  call main's API, and drop the duplicate status fields and the second
  BarrelWait row in `common_table`. Keep the Kongo catch guard until step 1
  is fixed; main's Kongo Jungle 64 barrel likely has the same x730 question.
- Main names every Versus stage in `make_boundary.py` STAGES and
  `melee_lib::Stage` (`KongoJungle` 0x05, `JungleJapes` 0x0C,
  `KongoJungle64` 0x1E). This branch added `KongoJungle` (0x05, "kj") and
  `JungleJapes` (0x0C, "jj") to the same places; take main's entries and keep
  this branch's only where main has none (the "kj"/"jj" boundary prefixes,
  scene_stage and initial_state wiring, `config`, `melee-replay`, `explore`,
  `melee-platform` match arms). `harness/boundaries.toml` is a union: keep
  `start_jj_fox_marth4` (seed 2835597521) and `start_kj_fox_marth4`
  (seed 2146705218) next to main's entries.

## Commands

```sh
CARGO_INCREMENTAL=0 cargo build --release -p melee-sim
# gate from a worktree against the main checkout's data
MELEE_DATA_ROOT=/Users/nikhilunni/Projects/melee target/release/melee-sim gate \
  /Users/nikhilunni/Projects/melee/harness/scenarios/corpus_v3_kj_fox_marth4_e0c8363c7_p1.toml
target/release/melee-sim triage <same toml>
cd harness && MELEE_DATA_ROOT=/Users/nikhilunni/Projects/melee uv run python explore_batch.py \
  ../target/kj/explore_kj3 60 120 --boundary start_kj_fox_marth4 --samples 6 --jobs 6
cd harness && uv run python record_many.py scenarios/<name>.toml --jobs 6
cd harness && uv run python asm.py fn_801D8134 --fused
```

## Boundaries and recordings

- Boundaries `start_jj_fox_marth4`, `start_kj_fox_marth4` (savestates in the
  main checkout's `harness/roms/`, entries in `harness/boundaries.toml`).
- Explorer scenario TOMLs (all `corpus_v3_jj_*` and `corpus_v3_kj_*`) are in
  the main checkout's `harness/scenarios/` (untracked there except those
  committed on this branch); traces are in its `harness/traces/`. Round
  reports: `target/kj/explore_jj1`, `explore_kj1`, `explore_kj2` in the
  worktree (summary.md and triage files).

## Gotchas

- The Kongo event timer is drawn at match start (fn_801D542C); the first event
  is at tick 1664 in explorer matches, so short scenarios never see one.
- The box event is never drawn with items off: its weight counts only when
  `it_8026D324(It_Kind_Box)` allows boxes (grkongo.c:1191).
- Event models are preloaded hidden (`load_reserved`) and recreated with
  dynamic scheduler tags; `frame.rs` decodes `SceneStage::Kongo` through
  `kongo::procs::map_callback`.
- `Assets::stage_key_bank` maps bank 0 to the common bank (also used in the
  stage anim tick); rebase kept main's particle-bank loading in
  `initial_state/particles.rs` over this branch's `register_bank`.
- Worktree tests that read `harness/roms` fail (no data in the worktree);
  `melee-gr`'s `real_battlefield` is one.

## Divergences outside the area

None found. Only `ftCoD5A30` (revival platform reaches floor) came up once on
Kongo Jungle, failing closed.
