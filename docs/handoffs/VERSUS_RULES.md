# Versus rules: what is left (wave 26)

Ported and witnessed: Time (2-4 players, 2v2), the team stock steal,
handicap and Damage Ratio, timed stock team battles, Lightning, Slo-Mo,
Super Sudden Death, Stamina up to the HP-out KO. Details and witnesses:
`docs/PORT_NOTES/VERSUS_RULES.md`. Every boundary below exists in the main
checkout (`harness/roms`, `harness/scenarios`, traces recorded).

Still guarded, in the order I would take them:

1. **Stamina's HP-out KO** (`ftCo_800C8C84` -> `fn_800C8E74`, ft_0C8C.c):
   x2224_b2 and its state family (ftCo_Damage.c:1003/1096, ftCo_Fall.c:34,
   ftCo_Wait.c:37, ftCo_DownBound.c:304, ftCo_DownDamage.c:31-57,
   ftCo_DamageIce.c:314/439, ftCo_FallSpecial.c:34, ft_081B.c ledge grabs,
   ftcoll.c:1589 grabs), PlCo +6D8 for knockback, and the scene's
   fn_801B9850 GObj whose fn_801B96E8 proc slows the game to 0.4 for 100
   ticks (gm_801B97C4; `frame_timer` already takes a speed request) and
   ends the match 100 ticks after one player is left. First witness:
   `corpus_v3_fd_stamina_fox_marth_ef89b3e70_p0` (tick 3678: a hit in
   EscapeN takes Marth to DamageFall). Note every percent-add site in
   melee-ft is a Fighter_TakeDamage_8006CC7C: the guard runs after each
   fighter proc (`stamina_spent`).
2. **Coin** (boundary `start_fd_coin1_fox_marth`): the coin item
   (It_Kind_Unk4 = 0x9E, it_2E5A.c), spills on every hit
   (Fighter_UnkTakeDamage_8006CC30 -> ftCommon_8007EA90 -> it_802E5F8C,
   HSD_Randf), pickup (ft_8007C77C on alternate frames by slot parity,
   lbColl_80007B78, efSync 0x432), the death payout (ftCo_800D4E50),
   container coins (it_8026F8B4), coin standings (fn_8016588C x5 == 2).
   Score popups and no stock loss already follow match_kind.
3. **Giant / Tiny** (boundaries `start_fd_sp_giant_fox_marth4`,
   `..._tiny_...`): model scale 1.8 / 0.35 needs ftCo_800D105C's
   ftCo_800CF6E8 and each kind's CalcIndiviParam (guard in
   `Fighter::prepare`); the ratios and scale are already set up.
4. **Invisible** (permanent Cloaking Device, x2226_b3/b4) and **Single
   Button** (gm_8016B0FC input remap): boundaries gate idle.
5. **Camera** mode: the boundary driver could not reach its CSS.
6. A Sudden Death among only some players (3-4 player Time ties):
   `Match::sudden_death` refuses it.
