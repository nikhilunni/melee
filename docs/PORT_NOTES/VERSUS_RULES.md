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
- Slo-Mo (game_speed 0.5): the tick trace's first row after the boundary is
  a frame with no scene tick (retail's seed is unchanged at tick 1, and
  tick 2 holds the port's tick-1 draws: start_fd_sp_slomo_fox_marth4_cold).
  The recorder does not mark such frames yet; guarded. Stamina's KO slows
  the game to 0.4 for 100 frames (gm_801B97C4), so it waits on the same.
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
