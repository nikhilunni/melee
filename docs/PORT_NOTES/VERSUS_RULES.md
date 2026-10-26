# Versus rules beyond stock (wave 26)

The VS rules screen's other settings, as `gm_80167BC8` (gm_1601.c:3570-3680)
turns `GameRules` into `StartMeleeRules`. `make_boundary.py` pokes the save
data's rules (`GameRules` +2 mode, +3 time, +5 handicap, +6 damage ratio) and
each door's handicap on the CSS, so the menus build the match; the summary
and sidecar read the match's rules back (`match_kind`, `time_limit`, x30,
each player's ratios). The savestate importer checks them against the
scenario and refuses any major scene but `GM_VS` (the Special Melee modes).

## Time (`--mode time`)

Retail facts (decomp, confirmed by the witnesses below):

- `ftCo_800D34E0`: only a stock match (`gm_8016B094`) takes a stock; a fall
  always counts (`Player_UpdateKOsBySlot`, or `Player_IncSuicideCount` with
  no source) and the player always revives.
- The HUD scores a fall (`ifStatus_TriggerStockLoss` when `match_kind != 1`
  and `x2_5`): `if_802F7AF8`/`7BB4`/`7C30`/`7D08` create popup GObjs
  (GObj_Create(14, 15, 0)) between the KO's quake model and its blast; each
  lives until its IfAll `ScInfStc_scene_models` animation ends, and an own
  KO's "-1" adds a second one past frame 12 (`fn_802F7994`). Modelled in
  `melee_ef::score_popups` for the GObj addresses.
- Standings (`gm_80166378`): score = KOs - (falls - SDs) + SDs * xC (xC = -1
  by default), self/teammate KOs counting as both; `Player_80033BB8` ranks
  by it (team sums in a team battle). Timeout: one top player (team) wins,
  several go to Sudden Death. gm_SetupSuddenDeath keeps the tied leaders
  alone (the others' slots go to Gm_PKind_NA, ports kept), so the new
  scene's setup draws are for their fighters only (`Match::sudden_death`).
  Witness: `make_boundary.py --sudden-death-hold 3:-0.8742
  --sudden-death-fighters 3` plays a four-player one-minute match from the
  boundary it saves with P4 holding left, through TIME! and the tie screen
  (A), and saves the Sudden Death's first frame too:
  `w27_fd_time1_tie3of4_walkoff` (Captain Falcon self-destructs, Fox, Marth
  and Falco tie at 0; 3839 ticks), `w27_fd_time1_tie3of4_sd` (+ `_cold`,
  the three-player scene through the Bob-omb rain to GAME, 1635 ticks), and
  `melee-lib/tests/match_endings.rs` chaining both through the public API.

Witnesses: `start_fd_time1_fox_marth`, `start_fd_time1_fox_marth_falco_captainfalcon`,
`start_fd_time1_teams_fox_marth_falco_captainfalcon` (and `_cold`), explorer
samples `corpus_v3_fd_time1_*` played to TIME!.

## Coin (`--mode coin`)

`gm_8016B0B4` is true in a Coin match. The fighter side
(`melee_ft::fighter::coins`):

- Every Fighter_UnkTakeDamage_8006CC30 ends in ftCommon_8007EA90: the
  damage (fctiwz) in coins' worth spills from it_802E5F8C. With knockback
  the launch is ftCo_Damage_CalcAngle of kb_applied as the victim stands,
  mirrored when dmg.facing_dir_1 > 0 (double: 3pi or pi minus it, frsp), at
  that speed from dmg.x1854_collpos (the strongest log entry's position);
  without, straight up at PlCo +5C0 from the camera bone. The worth is
  credited to the source player (Player_SetUnk98). The spill draws where
  retail draws it (inside ProcessHit, before the reaction); the scene makes
  the coins after the proc, before the proc's item requests. The other
  8006CC30 callers (CaptureCut, Thrown, a captor launched, the barrel, DK's
  cargo) stay guarded (`refuse_unwitnessed_coin_spill`).
- A fall (ftCo_800D4E50, last in every death entry, not for x221F_b4):
  half the purse leaves it (Player_SetCoins, Player_SetUnk9C), the spill's
  rank is read after that, at most PlCo +5C8 coins spill at PlCo +5CC from
  the blast zone (left: angle 0, right: pi, bottom: pi/2), or, for a star
  or screen KO, from cur_pos as coins no fighter can take (state 1, no map
  collision). The scene pays out after the entry's quake and blast.
- it_802E609C: it_802E61C4 draws the worth once (count times the rank's
  x2C multiplier, plus up to x24 percent), then each coin takes the largest
  tier the rest covers, its launch speed times x1C and it_802E628C's draw.
  Each coin: Item_80268B18, it_802E6380 (tier, timers, scale, box, pickup
  sphere), it_802E66A0, it_802750F8 (`it-coin`'s `launched`). it_802E6658
  (58 coins out: the last one removed) has no witness and is guarded.
- The coin (`it-coin`, it_803F8BD8): flight, bounces (it_802E5AC4: a bounce
  is no landing; Add_xy onto zero vectors makes -0 positive), rest, the
  lifetime and the two pickup delays (x10 others, x14 the owner).
- Pickup (ft_8007C77C, Fighter_8006CB94 between the fighters' hits and the
  items'): on frames whose parity (gm_80479D58.unk_8, `scene_frame`) is the
  player's, a fighter not intangible takes each open coin one of its two
  spheres (x1614, ft_8007C6DC at link 9) swept past (lbColl_800067F8,
  `melee_coll::sweep`): its worth to the purse, effect 0x432 (generator
  0x145 at the contact) and Item_8026A8EC at once, whose efLib_DestroyAll
  leaves the generator insertion cursor at the tail before the next coin.
- Score: fn_8016588C x5 == 2, the coins held; the standings rank by it.
  A container's coins (it_8026F8B4) need items on and stay guarded.

Witnesses (boundary `start_fd_coin1_fox_marth`, 60 s): explorer samples
`corpus_v3_fd_coin1_fox_marth_*` played to TIME!, three of them with star or
screen KO payouts.

## The team stock steal

`fn_8016B918` (gm_16AE.c:546-590): a team battle's player out of stocks
whose fighter sleeps presses Start; the first teammate (slot order) with more
than one stock gives one. `ifStock_802F7EFC` starts the icon's flight on the
giver's HUD (no loss explosion for it), clears the taker's x204 flag;
`ifStock_802F8298` flies it ten frames (`lbVector_8000DE38`'s quadratic,
particles 0xFC on frame 1 and 0xFF on frame 10, efAsync 0x475/0x476) and sets
the flag again. The taker revives at once (`fn_8016719C`). Witness:
`teams_steal_fd_fox_marth` (Fox walks off FD twice, presses Start, takes
Marth's stock).

## Handicap and Damage Ratio

`ftColl_80079AB0` / `80079C70` multiply the launch by the match's x30, the
attacker's attack ratio and the victim's defense ratio (three fmuls after the
formula, 80079B58-60), before the cap. `lbl_803B7930` gives the ratios by
handicap (9 is 1.07 / 0.9346, not 1.0). The logged hit carries its
attacker's ratio (an item's fighter owner's, else 1.0); a thrown fighter's
hit under handicaps (logged under its thrower) stays guarded.
`ftColl_80079EA8` (the barrel) takes no ratios. Witness boundary:
`start_fd_handicap9_1_ratio15_fox_marth4`.

## Special Melee

`make_boundary.py --special <mode>` walks VS. Mode > Special Melee; each mode
is its own major scene (GameModeKind), which the importer maps to
`SpecialMelee` and checks against the scenario's `special`.

- Lightning (`gmlightning.c`, game_speed 1.25): only the scene's frame
  period changes (gm_16AE.c:1985). Ported; explorer witnesses exact.
- Super Sudden Death (`gmsupersudden.c`, PlayerInitData x12 = 300): every
  player starts and revives at 300% (FighterMatchInfo.x6, read by
  fn_8016719C). Ported; explorer witnesses exact.
- Slo-Mo (game_speed 0.5): polls stay at 1/60 s (lb_80019628 takes the
  shorter period), so gm_801A4D34 makes a scheduler pass per VI, and
  lb_80019900's game timer is due on every other one; the others pause
  every p_link (gm_803DA8C8) and skip the frame callback: the tick trace
  records them as unchanged rows. `crate::frame_timer` models it, with
  fn_8016E730's speed request taken up after the creation pass. The tick
  validator and pad schedule check accept such rows (the pad is dequeued,
  the game's status not renewed). Ported; explorer witnesses exact.
- Stamina (`gmstamina.c`): one stock and 150 hit points
  (Player_SetOtherStamina, MoreFlagsBit2 -> fp->x2225_b7). The HUD shows
  Player_GetRemainingHP and shakes as it falls (ifstatus.c:577), and
  knockback reads PlCo +6D4 instead of the percent (ftColl_GetDamageCount,
  ftcoll.c:2363), +6D8 once the fighter is out.
  The KO (`melee_ft::fighter::stamina`): every Fighter_TakeDamage_8006CC7C
  ends in ftCo_800C8C84, which marks a fighter whose hit points are gone
  (x2224_b3). With a knockback hit applying (dmg.kb_applied) fn_800C8_inline
  knocks it out at once (x2224_b2: Fighter_ResetInputData, color animation
  0x7A, the x4C_sfx->xC cry, gm_80167470); the port checks that hit before
  its reaction, on the percent it will leave. Otherwise ProcessHit's
  ftCo_800C8D00 (fighter.c:2963) does, and then drops the fighter:
  ftCommon_8007DB58, a heavy item dropped (ftCo_8009750C), DamageFall in the
  air. Out, the fighter reads no input (fighter.c:1784), takes no ledge
  (ft_081B.c:138), lies in DownWait / DownDamage without counting down,
  takes any prone hit in DownDamage (ftCo_8009F0F0), gets 0x7A back when its
  primary color slot empties (ftcolanim.c:254) and is no CPU target.
  The scene (`melee_lib::stamina`): gm_801B97C4 slows the game to 0.4
  (Player_UpdateMatchFrameCount first); fn_801B96E8 (the last proc, s_link
  0x15) returns it to 1.0 after 100 ticks and, 100 ticks after at most one
  player (team) is left, ends the match (gm_8016B328: OUTCOME_TERMINATED,
  then the ordinary GAME! freeze). A proc's gm_SetGameSpeed reaches the
  frame timer at the next pass's lb_80019894 (`FrameTimer::request`). A
  match-start boundary predates fn_801B9850 (gm_804975F8 still zero), so the
  import starts the rules fresh; a later one reads gm_804975F8 (0x1C bytes:
  the counter, four MSL `bool` ints, four u16 slow-motion counters).
  Guarded (no witness): DownSpot (a grounded KO, ft_08A1.c:74),
  ftCo_Damage_Anim / Coll's branches (ftCo_Damage.c:1003/1096), DownDamage
  ending airborne, a KO with a grab link or by an unlaunched hit (inlineB2),
  and a knocked-out fighter entering any state outside DamageFall, Damage*,
  DownBound/Wait/Damage (`run_stamina_rules`), which covers Wait, Fall,
  FallSpecial, ledges and grabs. Witnesses:
  `corpus_v3_fd_stamina_fox_marth_ef89b3e70_p0` (an EscapeN hit to
  DamageFall at tick 3678, the slowdown, a prone hit, the end at 3927) and
  `..._e89a89d0e_p0` (a blast-zone KO).
- Giant and Tiny (`melee_ft::fighter::model_scale`): every fighter at
  Player_GetModelScale 1.8 / 0.35. ftCo_800D105C's ftCo_800CF6E8 scales the
  attributes along PlCo's curves (Fighter_804D6524, ftCo_CalcYScaledKnockback
  in `melee_coll::damage::y_scaled`), the ECB weight included (ft_80081B38);
  knockback (ftCo_Damage.c:139), hitbox damage including throw boxes
  (ftColl_8007ABD0), Walk's acceleration (ftCo_Walk.c:65), the soft-landing
  speed (ftCo_800D0EC8) and fighter-owned item counts (it_80272460) follow
  the scale, and it_80274594 scales the Illusion and held articles by the
  owner's. The throw rate reads the dat's weight, not the scaled one
  (ftCo_Throw.c:168-172). Marth's Dolphin Slash startup is
  ftCommon_8007CF58 (over-drift friction above the drift maximum), which a
  Tiny Marth reaches. Kinds whose LoadSpecialAttrs, multi-jump table or
  cargo attributes also scale (Link, Mewtwo, Pikachu, Samus, Kirby,
  Jigglypuff, Donkey Kong) are refused (`SCALES_SPECIAL_ATTRIBUTES`), as are
  a scaled HeavyGet, scaled pickup boxes and the per-kind scale guards in
  the specials. Witnesses: `corpus_v3_fd_sp_giant_fox_marth4_*` and
  `corpus_v3_fd_sp_tiny_fox_marth4_*` (three explorer samples each).
- Invisible sets x2226_b3, a permanent Cloaking Device (x2226_b4 at every
  spawn, ftCo_800C89A0). Its readers change only what is drawn: the cloak's
  color slot x508 runs PlCo table 7 (0x7B..0x80: opcode 23 toggles x2226_b5
  and the x2134 alpha, read by ftmaterial.c / ftdrawcommon.c / lbshadow.c;
  sounds 299/300 draw nothing), x2084_b4 feeds the bonus tally, ftlib.c:807
  the shadow, ftLib_80086F4C the name tag, and Fighter_TakeDamage_8006CC7C
  still takes damage (`!b4 || b3`). A CPU's targeting reads the slot
  (ftCo_800A0F00 via ft_80087A18) and stays guarded. Witnesses:
  `corpus_v3_fd_sp_invisible_fox_marth4_*`.
- Single Button (gm_8016B0FC, StartMeleeRules +2 bit 4): the pad keeps A
  alone and no analog shield (fighter.c:1875; a CPU keeps A and X/Y), and
  the C-stick neither throws a battering item (ftCo_800DF50C) nor drops
  from a ledge (ftCo_800DF72C). Witnesses:
  `corpus_v3_fd_sp_single_button_fox_marth4_*`.
- Camera mode is GM_CAMERA_MODE (0x0A, not GM_CAMERA_VS 0x2A): its camera
  screen (state 0, GS_CAMERA_VS) takes A, then CSS 1, SSS 2, match 3, and
  its CSS starts only with a controller in port 4, the camera's
  (`make_boundary.py --special camera` with `MELEE_SI_PORTS=0,1,3`; every
  recording from the boundary needs the same). Boundary
  `w27_start_fd_sp_camera_fox_marth4` (seed 3929830345, its 600 idle
  ticks recorded). The match (gm_PrepCameraModeVSScene) is an untimed Time
  match: x5_0 makes gm_GetMatchOutcome always NONE, x1_2 / x1_3 replace
  the countdown and GO banners with element 8, x2_5 off (no score
  popups), x4_0 and x3_1 off, every player xD_b3; gmCamera_801A31FC
  creates two UI GObjs and the SIS texts at match start, and port 4's
  pad drives the snapshot UI each frame (gmCamera_801A3098: Z pauses for a
  snapshot through gm_801A4634(3), cmSnap). Still guarded: no part of it
  is ported.

Boundaries: `start_fd_sp_<mode>_fox_marth4` (+ `_cold`) for lightning,
slomo, super_sudden_death, giant, tiny, invisible, single_button, and
`start_fd_stamina_fox_marth`.

## Still guarded

Bonus matches; Camera mode (above); in a Coin match, it_802E6658's
removal of the last coin with 58 out, a container's coins (it_8026F8B4,
items on) and the spills of the other Fighter_UnkTakeDamage_8006CC30
callers (CaptureCut, Thrown, a launched captor, the barrel, DK's cargo).

## Standings and timed team battles

`standings::ranks` is gm_80166378 for every mode the port plays: a stock
match's score is the stocks left, or for an eliminated player its
elimination second from 0xFF000001 (Player_UpdateMatchFrameCount, kept in
`InitialState::eliminated_at`); a team battle ranks teams, fn_80165E7C
combining a stock team's members (the best eliminated score until a member
with stocks replaces it, then sums). Player_80033BB8 and the TIME! result
both read it. Witness: `doubles_timeout_fd_fox_marth` (boundary
`start_fd_doubles_timer1_fox_falco_marth_captainfalcon2`).
