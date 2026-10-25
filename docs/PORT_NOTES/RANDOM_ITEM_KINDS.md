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
- Heart Container, Maxim Tomato, Food: not ported. Eating needs the fighter's
  consumable pickup (ftpickupitem_8009447C) and the heal (Fighter_8006CF5C,
  applied in Fighter_ProcessHit), which still fail closed.
