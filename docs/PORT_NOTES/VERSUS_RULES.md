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
  several go to Sudden Death; a Sudden Death without every player stays
  guarded (`Match::sudden_death`).

Witnesses: `start_fd_time1_fox_marth`, `start_fd_time1_fox_marth_falco_captainfalcon`,
`start_fd_time1_teams_fox_marth_falco_captainfalcon` (and `_cold`), explorer
samples `corpus_v3_fd_time1_*` played to TIME!.

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
  (Player_SetOtherStamina, MoreFlagsBit2 -> fp->x2225_b7). Ported: the
  HUD shows Player_GetRemainingHP and shakes as it falls (ifstatus.c:577),
  and knockback reads PlCo +6D4 instead of the percent
  (ftColl_GetDamageCount, ftcoll.c:2363). Guarded: the KO when the hit
  points run out (ftCo_800C8C84 -> fn_800C8E74, ft_0C8C.c), which sets
  x2224_b2 and sends the fighter into DamageFall / DownSpot from a dozen
  states (ftCo_Damage.c:1003/1096, ftCo_Fall.c:34, ftCo_Wait.c:37,
  ftCo_DownBound.c:304, ftCo_DownDamage.c:31-57, ftCo_DamageIce.c, ledge
  grabs off in ft_081B.c, grabs off in ftcoll.c:1589), uses +6D8 for
  knockback, and slows the game to 0.4 for 100 ticks (gm_801B97C4,
  fn_801B96E8; `crate::frame_timer` takes the request). Witness to resume
  from: `corpus_v3_fd_stamina_fox_marth_ef89b3e70_p0` (KO at tick 3678,
  EscapeN to DamageFall); exact witness `..._e89a89d0e_p0` (a blast-zone
  KO).
- Giant and Tiny scale every fighter (1.8 / 0.35): ftCo_800D105C's
  ftCo_800CF6E8 and the kinds' CalcIndiviParam (ftchangeparam.c:178-195)
  are not ported; guarded in `Fighter::prepare`.
- Invisible sets x2226_b3, a permanent Cloaking Device (x2226_b4 at every
  spawn, ftCo_800C89A0), whose readers (ftlib.c:807, ftcolanim.c,
  ft_0892.c, fighter.c:1202/1562) are not ported; Single Button remaps
  input (gm_8016B0FC, ft_0DF1.c). Both boundaries gate idle; guarded.
- Camera mode: the boundary driver could not reach its CSS (likely locked
  in the boundary's save data).

Boundaries: `start_fd_sp_<mode>_fox_marth4` (+ `_cold`) for lightning,
slomo, super_sudden_death, giant, tiny, invisible, single_button, and
`start_fd_stamina_fox_marth`.

## Still guarded

Coin and Bonus matches (`gm_8016B0B4`: coins from hits, the coin item, the
coin HUD; boundary `start_fd_coin1_fox_marth`), the Special Melee modes
above, and a Sudden Death among some players.

## Standings and timed team battles

`standings::ranks` is gm_80166378 for every mode the port plays: a stock
match's score is the stocks left, or for an eliminated player its
elimination second from 0xFF000001 (Player_UpdateMatchFrameCount, kept in
`InitialState::eliminated_at`); a team battle ranks teams, fn_80165E7C
combining a stock team's members (the best eliminated score until a member
with stocks replaces it, then sums). Player_80033BB8 and the TIME! result
both read it. Witness: `doubles_timeout_fd_fox_marth` (boundary
`start_fd_doubles_timer1_fox_falco_marth_captainfalcon2`).
