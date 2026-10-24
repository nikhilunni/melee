# Every Versus stage named; the N64 stages ported

Wave 13. Dream Land N64 (groldpupupu.c) was already the port's `DreamLand`
(GrKind OldPupupu, `melee_gr::pupupu`). This adds every Versus stage to the
stage tables, and ports Yoshi's Island N64 (groldyoshi.c) and Kongo Jungle
N64 (groldkongo.c), with Kongo's barrel cannon on the fighter side
(ftCo_BarrelWait.c).

## Every Versus stage

- `harness/make_boundary.py` STAGES lists all 29 Versus StKinds
  (gr/forward.h), spelled as slp `ids.rs` `stage_name` spells them.
  `boundary_script.py` unlocks all eleven unlockable stages (0x07FF, as
  gm_8016468C does); music rule 7 (Ground_801C24F8 -> gm_80164600) is the
  only match state the stage bits reach, and no ported stage uses it.
  Savestates made before this have 0x00C0 (Battlefield and FD).
- `melee_lib::Stage` has every Versus stage (`Stage::VERSUS`, StKind order,
  with name, display name and StKind); `Stage::PORTED` is what a match can
  start on and what the menus offer. Starting on any other stage returns
  `StartError::UnportedStage`, and scenarios and replays report the same
  message ("Kongo Jungle (KongoJungle, StKind 0x05) is not ported yet").
  A boundary on an unported stage records on retail and stops at the gate
  with that message.

## Yoshi's Island N64 (GrOy.dat)

- grOldYoshi_8020E79C creates maps 0, 1, 4, 5, 2, 3; every map has a gobj
  proc. Setup draws: one (the guest's first wait, grOldYoshi_8020EFCC).
- Clouds, map 2 (grOldYoshi_8020EAFC / 8020EC10): the archive binds
  collision joints 0..2 to descendants 1, 5, 9 (grOy_803E6574), bound from
  the unanimated pose before animation 0's frame zero. Each cloud's joint
  `cb_0` is fn_8020F2A8: a fighter's CollData (x34 b1234 == 1) marks the
  cloud carried. `melee_mp::JointCollisionHandler`'s `user_data` is now a
  writable word in the joint's slot (the `Ground*` the stage registered),
  which the cloud proc reads and clears.
- A carried cloud sinks to 6 (grOldYoshi_8020F31C: braking inside the
  stopping distance, 0x8020F370 fmadds), counts past 120 and fades
  (animation 1's materials on its subtree, the stage-bank puff 30000 at its
  JObj, joint removed at count 30, hidden and removed at the fade's end),
  stays gone past 180, and returns (animation 2's materials, shown at once,
  joint back at count 30). `BackgroundAnimation::play_material_subtree` and
  `material_subtree_ended` are grAnime_801C7FF8 / grAnime_801C83D0 with the
  material mask.
- The guest, map 3 (grOldYoshi_8020F088): while waiting it restarts its
  animation and draws a height every frame (HSD_Randf; +0x18 is read with
  `lha`, so 30, not the decomp's float); past the wait it draws one of five
  descendants (HSD_Randi(5)), shows it until the animation ends, then draws
  the next wait (3000 + HSD_Randi(1000)).
- Map 4's proc is Ground_801C2FE0 and lb_800115F4; lights and fog are map 4
  (flags 0xC0000000).

## Kongo Jungle N64 (GrOk.dat)

- grOldKongo_8020F46C creates maps 0, 3, 1, 2. Setup draws: three (the
  barrel's turn wait and drift, the bird's wait). GrOk.dat has no particle
  bank; `ParticleBank::empty` stands in.
- Platforms, map 3: grOk_803E6580 binds collision joints 0 and 1 to its
  descendants 1 and 2; its proc moves them (Ground_801C2FE0) and ends with
  lb_800115F4.
- The barrel, map 1 (stageGObj1_GObjProc 0x8020F888, `melee_gr::oldkongo`):
  the aim controller (brake and wait, spin up for 90..110 frames, then turn
  to a weighted random aim, grOldKongo_80210650), the aim wrapped in double,
  the cannon's world position published (Ground_801C4D70), the path
  controller (the barrel animation's rate ramps 0.005 per frame up to 1,
  cruises 3000..6000 frames, stops 300..600) and the catch.
- The catch device registers at the countdown's end (grOldKongo_8020F4E8).
  Each fighter's grab proc tests it first (ftColl_8007BA0C, ftCo_800C0A28):
  within 15 of the cannon the barrel draws the shot delay
  (0x80210510 fmadds, 479..480 frames), plays animation 1 on its subtree 2
  and the dust 0x405 at the fighter, and the fighter enters BarrelWait
  (ftCo_8009EB18: invisible, intangible, every grab category excluded,
  PlCo +4DC/+4E4 camera box). The scene places it on the cannon each tick
  (ftCo_8009EC44) and relays A/B (Ground_801C4DD0) to fire early.
- The shot: animation 2, eleven frames of windup, then ftCo_8009EC70 a
  quarter turn past the aim (double add and wrap, x57.29578 fmuls), turned
  to the fighter's facing and truncated into x1848_kb_angle; knockback by
  ftColl_80079EA8 (ftColl_80079AB0's formula on the fighter's own percent),
  forced DamageFlyRoll. PlCo +5E0 frames pass before another catch
  (bury_timer_2, ftCo_800C0A98).
- The bird, map 2: hidden until its wait runs out; then animation 0 at a
  random height (0x80210168 fmsubs, 0x80210174 fmadds) from a random side.
  Camera_800307D0's bounds would move the entry further out; the bird only
  draws, so the port leaves X at +-200.

## Fail-closed

- Death while a barrel holds the fighter (BarrelWait's death2_cb,
  ftCo_8009EAF8 -> grOldKongo_802105C8): `unimplemented!`. The cannon stays
  inside the blast zone.
- A savestate boundary with a fading cloud, a showing guest, a held fighter
  or a flying bird: the importer refuses.
