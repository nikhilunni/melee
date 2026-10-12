# Mr. Game & Watch

Mr. Game & Watch (PlGw.dat, scenario name `GameAndWatch`) vs Fox on Final
Destination, boundary `start_fd_gameandwatch_fox4`. Crates: `ft-gamewatch`
(the fighter), `it-gamewatch` (his ten articles). He has no character effect
file (no `EfGwData.dat`).

## What is different about him

- **Flat model.** `ftGw_Init_OnLoad` sets `fp->x34_scale.z` to the width
  attribute (0.01). `Fighter_UpdateModelScale` (80067BB4) puts it in the
  root's x scale (`Capabilities::model_width`), so every bone's world z is
  flattened; the articles' recorded z positions depend on it. Hit tests
  against him go through `x44_mtx`
  (`Fighter_UnkApplyTransformation_8006C0F0`, `CombatState::flat_matrix`):
  hurt, shield, reflect and absorb volumes take `x44_mtx * bone` as their
  matrix and the fighter's depth as their z (`lbColl_80007ECC`,
  `lbColl_8000805C`, `lbColl_80007BCC`).
- **Normals are character rows.** Jab and rapid jab (341..344), down tilt
  (345), forward smash (346), neutral, back and up aerials (347..349) and
  their landings (350..352) reuse the common callbacks behind his own
  entries (`ENTER_JAB`, `ENTER_RAPID_JAB`, `ENTER_DOWN_TILT`,
  `FORWARD_SMASH`, `ENTER_AERIAL`). The down tilt takes no new attack
  instance and has no repeat latch; the three aerials land at the full
  attribute lag whatever L does (the up aerial reads the back aerial's lag).
- **Articles.** Each of those moves creates an article from accessory4 in
  one of his parts (`Item_AttachGameWatchArticle`): Greenhouse sprayer,
  Manhole, Fire torch, Parachute, Turtle, Spitball Sparky breath. Each
  lives while its owner stays in the move's rows; damage, death and leaving
  the ground in a grounded attack remove them all (`ftGw_Init_OnDamage`,
  `ftGw_Init_8014A538`). The hitboxes are the fighter's own script's.
- **Stepped animation** is in the animation data; nothing in the code
  treats it specially. `x2223_b1`, which OnLoad sets, has no reader in the
  retail fighter code.

## Specials

- **Chef** (353/354): accessory4 throws a food on the script's flag;
  `HSD_Randi(3)` over the five foods minus the last two thrown. Held B
  loops on `cmd_vars[2]`, a B press on `cmd_vars[1]`, up to five foods.
  The food (`it-gamewatch::chef`) flies on its own attribute entry, turns
  back off walls and lies spent after a floor, a hit or its lifetime.
  Its capsule is the command size, not size / scl: `it_8027CE64` runs
  `it_80274594` after the launch has made the hitbox (retail x1C 1.953 at
  scl 1.5), found by an explorer match where a food grazed an airborne Fox.
- **Judgment** (355..372): the entry draws the face before the motion
  change (`ftGw_SpecialS_GetRandomInt`: the enabled faces minus the last
  two), so it is deferred to `Fighter::finish_input`
  (`INPUT_RNG_ENTRY`). Each face is its own row and script; the sign is an
  article. Face 7's food (`it_8028FAF4`) needs items switched on
  (`it_8026D324`), so with items off retail makes none; the port does not
  model the items-on branch. One hop per airtime (`x2234`, cleared by
  landing).
- **Fire** (373/374): both entries leave the ground with jumps spent; the
  trampoline is a free article below TopN (8014DF4C: fnmsubs); the rise is
  the animation's TransN turned by the one-time lean (`ft_80085154`); the
  end is the special fall.
- **Oil Panic** (375..380): the absorbing bubble follows the script's
  `cmd_vars[0]`; absorbed hitboxes fill the bucket through
  `ftData_OnAbsorb` (the Catch rows) and add their truncated damage; at
  three levels the next use spills (the Shoot rows), whose hitboxes take
  `absorbed * x78 + x74` (two `__cvt_fp2unsigned`).

## Shared-engine work that came with him

- Absorbing bubbles (`melee-ft::fighter::absorb`): `ftColl_CreateAbsorbHit`,
  the absorb step of `ftColl_8007925C`, `ON_ABSORB`, and the item side
  (`ItemCore::pending_absorb`, the kind's `absorbed` callback).
- `ItemHitFlags::absorbable` is x42_b0 (word bit 19); bit 17 is x42_b2,
  "misses a fighter facing the same way", now tested where retail tests it.
- `ItemEvent::BoneGust` (the Manhole's `lb_800119DC` from its cover bone),
  `ItemLogic::PICKUP_READS_OWNER`, `ARTICLE_HITLAG_BEGIN`.
- efAsync 0x3FB from a landing command (the down aerial's landing).

## Not ported (fail closed)

- A held item stowed under the Manhole (`ftGw_AttackLw3_ItemManholeSetup`,
  ftgamewatchattacklw3.c:36-41): needs items on.
- The rapid jab's accessory over an item pickup
  (`ftGw_Attack100Start_Enter`): needs items on.
- `ftGw_SpecialS_GetRandomInt` with no face enabled (retail reads an
  uninitialised result).

## Frozen (DamageIce, shared)

Judgment 8 is an Ice hit; at knockback level 2 or 3 it freezes the victim
(`melee-ft::fighter::frozen`, ftCo_DamageIce.c, for any victim):
- the launch angle is bent upward (calcAngle inlined in ftCo_8008DCE0,
  8008DE38: the unit vector plus straight up, fmadds) and plays DamageFlyTop;
  then ftCo_DamageIce_Init: the timer is `x1838_percentTemp * PlCo +790`,
  the spin is drawn (`HSD_Randf`, fmadds), YRotN moves by co_attrs
  +154/+150, every dynamic bone returns to the animation (ftCo_8009E140,
  x2227_b6 keeps it so in ftCo_8009E7B4), one round capsule on XRotN, and
  a square box of the ice size around XRotN is the ECB (ftCo_800909D0; its
  first map pass runs after Fighter_ProcessHit returns, which has no map);
- the ice block is efAsync 0x415 (common model 0x25, AttachChild) and its
  particles use opcode 0xE9 (stepped dual colour, now in `hsd-particle`);
- DamageIce: reduced gravity in the air (PlCo +77C), friction on the
  ground, the timer and mash-out (`ftCommon_GrabMash`, +794/+798); in the
  air the box never lands before frame 3, so it lands through
  ft_800824A0; walls and ceilings bounce or shatter it (ftCo_DamageIce_Collide,
  fnmsubs at 80091804; retail tests the right wall twice, so a left wall
  takes the ceiling's y adjustment);
- a hit while frozen (ftCo_8008EC90 -> ftCo_8008DCE0(DamageIce) ->
  HitWhileFrozen) takes PlCo +714 of the damage and +718 of the
  knockback; OnHit2 takes `percentTemp * +79C` off the timer (fnmsubs)
  and fire empties it;
- the break-out (ftCo_80091854): efAsync 0x443 (generator 0x1F1), the hop
  (co_attrs +158, stick x * +15C), DamageIceJump for PlCo +7A4 frames.

Witnesses: `gamewatch_judgevar2_fd_fox4` (grounded freeze, landing,
timer), five explorer matches, `gamewatch_ice_{mashout,hit,firethaw,air}_fd_fox4`.
Not reached (explorer probes over 700 seeds found none): the bounce and
shatter (Final Destination's walls are under the ledges), a grounded block
leaving its floor (hits on the block barely move it: PlCo +718), a frozen
fighter leaving the top (ftCo_800D41C4/800D47B8, `unimplemented!`), a
cape or a throw release freezing (`unimplemented!`), the hammer thaw.

## An article destroyed by a hit and the generator cursor (2026-10-12)

`corpus_v3_fd_gameandwatch_fox4_e17844051_p0` diverged at tick 5464 on the
particle RNG only: Fire Fox's first launch tick burns him in his back aerial
with the Turtle out, and the burn overlay's children differed (retail 59 and
58 of family 2195 ahead of Fox's 2193 family; the port 354, 58, 57, 56
behind it). The same hit was exact in isolation
(`gamewatch_punish_firefox_bair_fd_fox4`).

hsd_8039D9C8 (generator.c:328-341) links a new generator in behind the
list's head when the insertion cursor (hsd_804D78F8) is null or the tail,
and behind the cursor's successor otherwise. The cursor is whatever the last
walk of the list left: the update pass (generator.c:980-1017) and the
destroy walks hsd_8039D4DC and hsd_8039D688 all end on the tail. So outside
the pass a new generator goes in behind the head, except when the cursor is
no longer the tail, which needs a list whose only generator was head and
tail at once and then got successors.

That is tick 5464. Before it the list is Fox's charge fire (3010, the head)
and one persistent generator (301). Fox's motion change destroys the head:
one generator left, cursor on it. His launch fire's parent goes in behind
the head, the hit's dust behind the cursor's successor (the fire), and then
ftGw_Init_OnDamage (0x8014A4CC, the take-damage hook) destroys the Turtle:
Item_8026A8EC's efLib_DestroyAll (item.c:1991) walks the article's joints
with hsd_8039D688 and parks the cursor on the tail, so the burn overlay
(efAsync 0x413, generator 0x37) goes in behind the head, ahead of Fox's
fire. The port removed the article after the proc and linked the burn
behind the fire; the pass then reached the two parents in the other order
and each emitted its children with the other's draws. In isolation the
stage's first generator (30000) is the head and another the tail, so every
insertion is behind the head whatever the walks do.

`articles::on_damage` now queues `EffectRequest::ArticleDestroyed` (as
Samus's beam and Link's hookshot and boomerang do). The match gates all 5766
ticks with 0 differing particle ticks and is in `CORPUS_V3_MATCHES`. The
other removers (the specials' own, at a motion's end) call Item_8026A8EC
inside his proc too and do not queue the walk; no recording separates them.

## The flat screen KO (2026-10-12)

`gamewatch_screenko_flat_fd_fox4` (directed, a display pass after every
tick; `SLIPPI_REPLAY_WITNESSES`, as the other cold-twin directed scenes
are): the pads of `corpus_v3_fd_gameandwatch_fox4_e45a17231_p2`, where Fox's
up smash sends him off the top at 2983, with `boundary_seed = 14` so that
ftCo_800D3158's roll (0x800D326C) is within PlCo +520 (16 in 100). Retail:
DeadUpFall at 2983, DeadUpFallHitCameraFlat (8) at 3034. The port gates all
3200 ticks with 0 differing particle ticks: the port of 01312e7b, made from
the asm, needed no change. To find such a seed, run the port's cold scenario
over seeds (a cold twin naming the recorded scenario as `expected`, with
`boundary_seed`): six of the first 56 take the screen KO.

`slp_ps_gameandwatch_falco_r112449_t8200` (the replay
`GAMEANDWATCH/Game_20190629T112449` from its own boundary, recorded by the
Stadium work of the same day) holds the console's own screen KO (6921, 6972)
and the port gates it exact as well.
