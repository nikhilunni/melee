# Handoff: wave 27 random item lifecycles (Hammer, Mushrooms)

Wave 27 ported the Ray Gun, Fire Flower, Super Scope, Starman, Cloaking
Device, Screw Attack, Metal Box and Bunny Hood (`docs/PORT_NOTES/
RANDOM_ITEM_KINDS.md`, "Shooters" onward). Two lifecycles were not started;
their boundaries are recorded and registered nowhere yet:

| Boundary (main checkout) | Seed | Item mask |
|---|---:|---|
| `start_fd_items_hammer` (+ `_cold`) | 4280688657 | `0xfffffff810000000` |
| `start_fd_items_kinoko` (+ `_cold`) | 2093931175 | `0xfffffff804000000` |
| `start_fd_items_dkinoko` (+ `_cold`) | 3284521034 | `0xfffffff808000000` |

The `make_boundary.py` logs with their `[[boundary]]` blocks are
`target/w27/mb_{hammer,kinoko,dkinoko}.log` in the wave-27 worktree; the
savestates are backed up in `~/melee-data/roms`. Their items have no
`it-*` crate, so the boundaries gate only until the first item spawns.

## Hammer (ithammer.c, ithammerhead.c, ftCo_Hammer*.c)

- Item (it_803F6150): rest 0, fall 1 (an empty hammer, xD4C 0, is removed
  by the floor pass it_8026DF34), held 2 (item anim 0, its swing hitboxes)
  and 3 (anim 1, headless: entered when xD4C reaches 0), drop flight 4,
  slope slide 5. PickedUp installs `entered_hitlag = it_80293D94` and calls
  it_802725D4. Joint 3 (the head) hides once xD4C <= 0.
- Pickup (ftpickupitem.c:182 -> ftCo_800C52F4): ftCo_800C5284 sets the
  timer x2330.x = PlCo +6AC and the head's fall-off time x2330.y =
  it_80293DCC (HSD_Randf < x8/100, then HSD_Randi(x4 - x0) + x0: two RNG
  draws at the pickup, or one), music ft_800880AC; then HammerWait on the
  ground, HammerFall in the air.
- The seven states (ftCo_HammerWait..HammerLanding, motions 307..313 in
  ftData_MotionStateList) mirror the cargo carry's (`melee-ft`
  `fighter/cargo.rs`): walk (ftWalkCommon), turn (PlCo +6B0 frames, a
  TransN rotation like CargoTurn's), knee bend (jump_startup_time, then
  HammerJump with ftCo_800CB110(gobj, false, 1)), fall (ft_80084DB0,
  ft_80082C74 -> HammerLanding), landing (PlCo +6B4 frames). Every entry
  keeps the swing's animation frame (ftCo_800C548C) with the skip flags of
  ftCo_800C54C4 while already in a hammer state, and runs ftCo_800C4E94:
  colour 106 (ftCo_800BFFD0 unless already shown), it_8026C16C(item, 1)
  and x2219_b4, a rumble.
- inlineB0 (every state's Anim): headless (x2338.x) -> ftColl_8007AFF8;
  else at each loop start (frame in [0, rate)) a new attack instance
  (ft_800892A0, ft_80089824, it_8027B070 on the item).
- Fighter_8006A360 (fighter.c:1636-1641) -> ftCo_800C511C each frame
  outside LightGet: the timer; the head's countdown in hammer states,
  whose end sets x2338.x and launches the head (it_80293E34 ->
  it_80299C48 Hammerhead from joint 3, direction from joint 1); at the
  timer's end the hammer is destroyed, music stops, ftCommon_8007D92C.
- HammerFall's IASA (ftCo_800C5DDC): with x2338.y (PlCo +6B8, set by
  ftCo_800C554C) A + L/R drops it. Platform drops (ftCo_800C60C8 ->
  ftCo_8009A184) need a platform stage. ft_8007C114 (ft_07C1.c) clears the
  item's hitboxes on hit; DamageIce has hammer arms.
- Holding it bars the item throw and attack routing the port takes for
  light items (`HELD_ITEM_STATES` must list the hammer rows only).

## Super / Poison Mushroom (itkinoko.c, itdkinoko.c, ftCo_Kinoko*.c)

- Touch (use kind 4 like the Starman, ftColl_80077C60's power-up branch,
  fail-closed today in `power_up.rs`): Fighter_SuperMushroomApply /
  ftCo_800D2490 etc. enter KinokoGiantStart(Air) / KinokoSmallStart(Air),
  or renew x2008 (PlCo +688, plus percent above +68C capped at +690) when
  already that size, or shrink back from the other size.
- The size change is gradual: fn_800D2890 / fn_800D299C interpolate
  x34_scale.y over the start animation (PlCo +678 / +67C ratios); every
  scale change goes through ftLib_SetScale-like updates: the model root
  scale, ftCo_800D105C (the port's `recompute_attributes` already applies
  `scale_curves` from `player.scale`; it must read a live scale instead),
  ft_80081C88 (ECB sizes and ledge snaps scale, not only the weight),
  hurtboxes, hitbox sizes, item pickup boxes (refused today:
  "ftCo_800D105C (ftchangeparam.c:200-215): a scaled fighter's itPickup
  boxes"), knockback and damage curves (Fighter_804D6524 x0/x4),
  animation rates (HeavyGet's), effects. The port assumes `player.scale`
  is fixed; this is the bulk of the work.
- The end (fighter.c:1500-1512): Fighter_SuperMushroomEnd /
  PoisonMushroomEnd and their states; ft_0877.c:278-296 and ft_0892.c
  read x2220_b5/b6.

## Left guarded in the ported kinds

- Slope slides of every item (`EnteredAir` and state-4/5 collisions):
  FD has no slopes; Battlefield's edges do not slope under resting items.
- Ray Gun: an absorbed ray. Super Scope: a shot along a floor, absorbed,
  reflected or shield-bounced shots, a charge no level affords.
- Screw Attack: a multijumper's screw jump (ftCo_JumpAerialF1.c:46).
- Metal Box / DamageScrew: a heavy item or grab pair when it applies.
- Transforming (Zelda/Sheik) under a Starman, while metal, cloaked or in a
  Bunny Hood (ftcommon.c:1317-1342).

## Stray recordings

Explorer bridges that were superseded and are not registered (safe to
ignore, never delete without the user): `explore_lgun2` leftovers and
about 133 `corpus_v3_fd_items_lgun_*` scenarios in the main checkout
beyond the 33 registered in `m5_gate.rs`.
