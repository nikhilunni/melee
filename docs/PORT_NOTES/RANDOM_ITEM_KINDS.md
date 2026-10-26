# Random item kinds (wave 26)

Each kind has a two-player Final Destination boundary with only that kind on
(`make_boundary.py --item-frequency very-high --items <Kind> --no-containers`),
gated warm and cold over 600 ticks, and explorer matches bridged to retail.

## Stomps (ft_80081A00)

A fighter whose airborne map pass came down onto an item with xDD0 b0 (shells,
Goombas) does not land: lb_80014638 tests its swept bounds (dmg.x1930, sampled
at s_link 0 and after the pass) against the item's xB54 box, which the item's
collision proc refreshes (it_8027574C) and creation and release reset
(it_80274DAC). The stomp sets the fighter's velocity blend (ftCo_800C703C,
PlCo x6CC/x6D0) and names the stomper and side on the item (xCFC, xCD0),
whose link 11 proc runs `jumped_on` before the hitbox refresh
(Item_80269B60). A pickup clears b0 (it_802742F4 -> it_80274C88).
x2223_b4 (Fox/Falco, Pikachu/Pichu, Zelda/Sheik, Mewtwo up-specials; Yoshi
and Bowser down-specials) suppresses the test; a motion change clears it.

The test is called from the shared air passes in `collision/air.rs`, the
damage pass (also in hitlag) and Fox's up-special passes. Any other airborne
collision path fails closed in the scene (`deliver_stomps`) when its fighter
came down onto a stompable item without the test having run.

## Engine pieces

- Item hits on other items record the victim with the rehit timer when the
  hitbox's x41_b4 is set (it_802706D0_sub3's mode 8).
- xCB8 (the hit's out direction) is set wherever retail sets it (ftcoll.c
  shield, clank and hurt contacts; itcoll.c item clanks and hits).
- Item clank sparks are made in call order inside the running item's link 13
  proc (efSync), not per item afterwards.
- A full hitbox clear keeps the capsules (`disabled_hitboxes`) for
  it_80272674, which the Red Shell uses.
- A Freezie's destroy effect 0x443 is the ice shatter (generator 0x1F1 at its
  root, scaled by its facing).

## Kinds

- Green Shell (itgshell.c): ported whole. Guarded as unwitnessed: a slope's
  slide (EnteredAir), a thrown shell glancing off a shield, a ground slide
  slowed under x8 by a wall.
- Red Shell (itrshell.c): ported; homing on the nearest fighter in play
  (ftLib_8008627C, fmadds distance). Guarded as unwitnessed: clanking, a
  thrown shell on a shield, a driving shell stomped, turned by a wall or off
  the floor, a slope's slide. Boundary `start_fd_items_rshell2`; the first
  capture `start_fd_items_rshell` diverges warm at tick 70 (Marth's entry,
  p1.cur_pos.y by 2 ULP) though its cold twin is exact, so it stays
  unregistered.
- Freezie (itfreeze.c): fall, slide, throw (breaks on any contact), the
  events that break it. Guarded: reflected, glancing off a shield, turned by
  a wall; states 4 and 5 (Icicle Mountain's polar bear). A frozen fighter KO'd
  off the top (DeadUpFallIce, ft_0D31.c:61-67) still fails closed on the
  fighter side (corpus_v3_fd_items_freeze_e0fcbde24_p1, unregistered).
- Bob-omb (itbombhei.c): the random item's fall, standing wait and turn
  before its first walk. Leaving the floor (it_8027DE18) is guarded.
- Mr. Saturn (itdosei.c): the random item's spawn fall (8), sitting (7) and
  look round (9). Leaving the floor while sitting or looking round and the
  slope slide before being held (10) are guarded.
- Motion-Sensor Bomb (itmsbomb.c): ported whole, with inert touches
  (Item.touched, xDCE b6: fighters' hitboxes and hurt capsules in ftcoll.c,
  other items in itcoll.c) and line sticking (xDCE b3). **An English game
  loads ItCo.usd** (it_8027870C), not ItCo.dat: the bomb's ECB box, scale
  and special attributes and the Cloaking Device's differ between the two,
  and only the .usd values match retail (the falling bomb's spun fixed ECB
  diverged at tick 247 with the .dat's). Guarded as unwitnessed: glancing
  off a shield, a slope's slide, an armed bomb losing its line (state 5).
- Heart Container, Maxim Tomato, Food (it-heal): the hop, fall and rest;
  Food draws its dish (HSD_Randi over itFoodsAttributes row 0's x0) once
  Item_80268B18 returns and is hold kind 2 (Item_802674AC). The pickup
  eats a use-kind-5 item (ftpickupitem_8009447C at LightGet's end, at once
  on an air catch, or on damage through LightGet's take_dmg_cb
  ftpickupitem_80094DF8): Fighter_8006CF5C adds to dmg.x18F0 and installs
  colour 8; Fighter_8006A360 heals a point a frame (also in hitlag) and
  clears colour 8 when done, and ftCo_800C0408's fallback reinstalls 8
  while points remain. Nana refuses healing items (x2222_b4). Food's dish
  model swap (it_80273318) is the presentation's and not modelled; its
  placeholder model has no attach joint, so `attachment_translation` is
  an Option. Guarded: dropped and sliding healing items, x1978 (a second
  held item), consuming the other use-kind-5 kinds.

## Shooters (wave 27)

Use kind 3 (it_8026B30C): a jab, side tilt or side smash on the ground
(ftCo_Attack_800CDD14, after ftpickupitem_80094790's pickup) and a neutral or
forward aerial (ftCo_AttackAir_CheckItemThrowInput -> ftCo_800CDDA0) shoot;
the vertical tilts and smashes, the dash attack and the other aerials are the
ordinary attacks with the item kept. An empty shooter is smash-thrown
(ftCo_AttackS4.c:123) and, after the launch draw spares it, a second
Randi(PlCo +41C) can knock it loose (Fighter_8006CDA4). Shots and the
switches between their ground and air rows (flags 0x0C4C5080) are in
`melee-ft/src/fighter/item_shoot.rs`, `item_flower.rs` and `item_scope.rs`.

- Ray Gun (itlgun.c, itlgunray.c): LGunShoot's accessory (ftCo_800CE1D4)
  fires on throw flag b0; the ray is a ray item like Fox's laser
  (Item_UpdateRayAnimation with divisor 7), reflected and bounced off
  shields; its destroy effect 0x436 (it_80278800's block_59C, generator
  0x153) and the muzzle flash 0x431 (common model 0x21, particle 308) are in
  melee-ef. Guarded: a slope's slide, an absorbed ray.
- Fire Flower (itfflower.c; its flames are It_Kind_L_Gun_Beam,
  itlgunbeam.c): the flame's draws (two Randf, a Randi for its generator)
  and the mouth flame (efSync 0x473 on the flower's bone 4) come between
  the holder's puff and the first flame's flash and spark, which go after
  the item requests (`effects_after_items`). The flower's model is posed in
  the hand once per article state (`ArticleSkeleton::load_states`), its
  authored RObj constraints left out (`load_joint_tree_without_constraints`:
  bone 4 lies below none of them, and a read below one fails closed); a
  looping state re-entered resets its tree (Item_80268D34's lb_8000B804).
  Six flames share a hit group; the flower renews it
  (`ItemRequest::Shot`).
- Super Scope (itsscope.c, itsscopebeam.c): the charge (fn_800D79B4,
  PlCo +5B8), rapid fire (PlCo +5BC shots per A press) and charged shots;
  the charge table is `melee_it::scope::ScopeCharges`. Guarded: a shot along
  a floor (itsscopebeam.c:153-184), absorbed, reflected or shield-bounced
  shots, a charge no level affords (it_80291DAC's fallthrough).

## Power-ups (wave 27)

- Starman (itstar.c): the bounce along the floor (it_8026DA08, a Randi(2)
  direction at spawn, the last creation draw); walls and ceilings are
  guarded. A touch (use kind 4, ftColl_80077C60's first branch) records
  the fighter, disables the hitbox, sets xC34 and gives x221D_b6 / x2004
  (ftColl_8007B7FC) under colour 0x6B; hits on the fighter are invincible
  contacts. The countdown (fighter.c:1486-1498) clears the colour if it owns
  the slot. The Super and Poison Mushrooms' touches fail closed
  (ftcoll.c:1127-1136).
- Cloaking Device (itspycloak.c): consumed by the pickup like a healing
  item (ftCo_800C88D4 with PlCo +7CC frames). Cloaked (x2226_b4) a hit adds
  no percent (fighter.c:2650, every hit path's `add_hit_percent`); its
  end (ftCo_800C8A64) returns from Fighter_8006A360, skipping that frame's
  animation callback. Its colour animations and refraction are
  presentation; a second cloak while cloaked fails closed.

## Screw Attack (wave 27)

- Item (itscball.c): falls, rests, held, thrown (state 3) until a floor;
  gone on a hit or a clank, it_80273030 on a reflect, a bounce off a
  shield or the victim. Its slope slide (EnteredAir, state 4) is guarded.
- Holding it (ft_800D2D0C), KneeBend's end enters ItemScrew
  (ftCo_ItemScrew_Enter: ftCo_800CB110 with PlCo +800 as the jump's
  multiplier, a Jump in its physics' first-frame skip, Fall's collision)
  and an aerial jump enters ItemScrewAir (ft_800D2E7C, ahead of the kind
  dispatch: the impulse times PlCo +800, ftCo_800CBAC4). A multijumper's
  screw jump (ftCo_JumpAerialF1.c:46) is guarded. Both rows stale as
  FtMoveId_Screw (0x5B) and spawn graphic 0x438 (ftCo_09F7.c block_66:
  model 0x22 on the joint, efLib_Cb_SetScale_FromParamX with the player
  scale times co_attrs x144).
- A HitElement_Scball hit outside motion groups 11 and 12 (ftCo_800D2FA4,
  after the bind check) takes DamageScrew / DamageScrewAir: straight up at
  screw_attack_launch_velocity, ft_80083B68 until a command sets
  cmd_vars[0], then AirCatchHit's landing; its end is ftCo_800968C8.

## Metal Box (wave 27)

- Item (itmetalb.c): falls and rests leaning with the floor (it_80276CB8),
  its ECB upright (xDCE b7). Consumed at the pickup (ftpickupitem.c:188),
  or broken by a hit (its DmgReceived): when the hit's fighter
  (it_8027236C) struck with x43_b0 (create_hitbox_3.item_hit_interaction,
  recorded as xDCF b6 by it_80270E30), that fighter turns metal through
  an owner request. Its slope slide is guarded; nothing enters state 3.
- Metal (ftLib_800871A8, ftCo_800C8348): the box's x0 frames and x4
  damage (fctiwz). ftCo_800D105C recomputes the attributes (Fighter_804D651C
  multipliers on the jumps, gravity, fall speeds, weight, ledge and wall
  jumps; the multijumps' rises through ftCo_800D0CBC) and ft_80081C88 the
  coll data's weight; a small quake. Knockback loses PlCo +6F0
  (ftCo_Damage_CalcKnockback). The timer runs at fighter.c:1525 and every
  percent add wears its health (int, fctiwz); ftCo_800C8540 ends it at 0
  frames, spent health or a death (ft_0D31.c:126). A transformation while
  metal, cloaked or under a Starman fails closed (ftcommon.c:1317-1340).

## Bunny Hood (wave 27)

- Item (itrabbitc.c): falls and rests; no thrown, hit or reflect logic
  (its ShieldBounced is itColl_BounceOffShield); the slope slide is
  guarded. Joint 3's visibility and the worn model's joint offsets and
  scale (ftCommon_8007FA00) are presentation.
- Picked up (ftpickupitem.c:185, ftCommon_8007FA58) it leaves the hand
  (Item_8026ABD8, no push, xC44 0) and hangs from the head (ftData x8
  +0x12) as x197C for its x0 frames (x2014, fctiwz); a second hood renews
  the frames and is gone. ftCo_800D105C applies Fighter_804D6520 (the
  dash, jumps, gravity, fall speeds, ledge and wall jumps; the multijumps'
  rises, then the metal's, ftCo_800D0CBC); animated aerial jumps rise
  times its +14 (ft_800851D0); every jab followup is enabled
  (ftAction_80071AE8). Its countdown (fighter.c:1514) and a death
  (ft_0D31.c:116) destroy it; a launch draws Randi(PlCo +418) < damage
  after the hand's draw (fighter.c:2700) and drops it from the head
  (xC44 1). Sounds, rumble and Pikachu's ear hook (ftCommon_8007F8E8)
  are not simulated.
