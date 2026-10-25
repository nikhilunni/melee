# Wave 26: the battering items' fighter rows and kinds

## Common attack rows with a battering item in hand

With a battering item (it_8026B30C == 2) held, the common up tilt, down tilt,
up and down smash, the five aerials and their landings run their ordinary
callbacks: none of ftCo_AttackHi3/Lw3/Hi4/Lw4.c, ftCo_AttackAir.c or
ftCo_LandingAir.c reads the item beyond the throw checks. The aerials' IASA
is DO_IASA (ftCo_AttackAir.c:147-155): ftCo_80095328's air throw first. The
side smash's C-stick throw (ftCo_800DF21C) holds for every kind. The
held-entry-only guard and its `held_on_entry` marker are gone; private rows
(Game & Watch, Link's down aerial) keep their refusals. AttackS3/S4,
AttackDash and Attack11 are never entered with a battering item: their
CheckInputs swing instead (ftCo_AttackS3.c:33-49, ftCo_AttackDash.c:30-42).

Witnesses: `w25_sword_lw_land_750_v01`, the directed
`w26_sword_nair_lw4_v01` (neutral air into LandingAirN, down smash, up
tilt, C-stick down throw) and ten Sword explorer matches.

## Kinds

fn_800CCEC4's rows are sword 0, bat 1, parasol 2, fan 3, star rod 4,
lipstick 5 (the motion states' order too).

- **Home-Run Bat** (itbat.c): the Sword's code with its own table (`it-sword`
  is generic over both). Six explorer matches from `start_fd_items_bat`.
- **Fan** (itharisen.c, ft_0CD3.c ftCo_800CD5A8): the holder drives the fan's
  own swing animations at its frame_speed_mul (cmd_vars[0] 1/2), the fan
  swells on a hit (deal_dmg_cb ft_800CD65C, PlCo +5A0 frames) and shrinks
  and rests on damage or leaving the floor (ft_800CD604). Its script's
  dust 0x434 is generator 0x14D. **Dropped fans fail closed on contact**:
  itHarisen_UnkMotion8_Coll branches on xD4C, which nothing writes for the
  fan and Item_80268B5C does not clear; retail raw traces show 0,
  0xF1F1F1F1, 0xD8BAB3BF, ... per pooled Item struct, so a dropped fan
  vanishes or lands by heap history. Seven explorer matches from
  `start_fd_items_harisen`, each cut before its first dropped-fan landing.
- **Star Rod** (itstarrod.c, itstarrodstar.c, ft_800CD914): shots from the
  attributes at spawn (xD4C); the swing's accessory4 plays the script's
  sound, and on throw flag b0 spawns a star at the rod's tip with flash
  0x42E (generator 0x13C, facing). The empty rod's puff (1032) is guarded.
  An item's destroy effect on a facing row (the star's 0x3F0) takes the
  item's facing (it_80278800's block_4FC); it took 1.0 before.
- **Lip's Stick**: still fails closed (random spawn and fn_800CCEC4). A
  drafted port (item, spore, ft_800CDB9C) reproduced the cold boundary only
  to tick 269, where the first falling stick's y is 4.69856 against retail
  4.71543 (the cause is not isolated yet; first suspect: it_80276278 aims
  the ECB with it_80274990, the root JObj's rotation, where the port reads
  only the item's spin). A thrown stick's first contact also releases a
  spore (it_8029A218). Both need porting before the kind can be admitted.
- **Parasol**: still fails closed; its ItemParasolOpen/Fall fighter states
  stay guarded in `parasol.rs`.

New Item Switch boundaries (`make_boundary.py --items <kind>
--no-containers`): `start_fd_items_{bat,harisen,starrod,lipstick,parasol}`;
the last two stay unregistered (the cold gate meets the unported kind).

Checks: full sweep against `f570da10` (3,729 scenarios in the main
checkout): zero regressions, one fix (`corpus_v3_ps_bowser_ness4_edb5ae9dd_p0`);
`public-v3.7` replays 108/108 complete, 964,782 frames matched.
