# Kirby

Kirby (PlKb.dat, scenario name `Kirby`) vs Fox on Final Destination,
boundary `start_fd_kirby_fox4`. Crate: `ft-kirby`. Data facts are in
`docs/KIRBY_DATA.md`.

**Status: stage 1 of three.** Kirby is registered and his movement, common
attacks, down throw, hit reactions, death and revival are exact. His own
specials (stage 2) and the copy abilities (stage 3) are not ported: every
special row and entry fails closed, as do his three airborne throws.

## What is different about him

- **Empty parts.** PlCo's part table gives Kirby 59 joints; his costume
  has 46. The other thirteen are conditional parts for the hats, which
  `ftParts_SetupParts` skips so the later joints keep their indices. The
  port's skeletons number joints by pre-order, so each empty part gets a
  stand-in leaf joint at its index (`fill_empty_conditional_parts`, applied
  to the costume description in `melee-lib`'s `CharacterArchive`). A
  savestate's NULL part joint leaves the stand-in alone
  (`saved_pose.rs`). Link and Young Link keep their OnLoad graft; nothing
  else has conditional parts.
- **A draw at every reset.** `ftKb_Init_OnDeath` (800EE5A0) sets the Stone
  form with `HSD_Randi(5) + 1`, at creation and at each revival.
  `CharacterCallbacks::RESET_DRAW` runs after `on_reset`;
  `RESET_DRAW_COUNT` adds it to the cold setup's fixed interval between a
  Game Start seed and the pre-music boundary seed (`setup_draws`), so a
  replay's or cold scenario's first Stone form is the console's.
- **The bump star.** `Fighter_procMap` calls `ftKb_SpecialN_800F1D24`
  after the state's collision callback (fighter.c:2494): a wall, ceiling
  or floor touched now and not on the previous tick spawns generator 0x206
  at that side of the ECB, then nothing for five ticks of contact
  (`u.kb.x60`). `CharacterCallbacks::AFTER_COLLISION`,
  `ft-kirby/src/bump_star.rs`. The effect queue is last in, first out
  (efAsync_Spawn links at the head): the star, queued after the landing's
  motion change, spawns before that script's dust, so it goes through
  `push_effect_after_issued_graphics`. Retail skips the call for a state
  without a collision callback; every row of the port has one, so the
  cooldown would tick there too. No scene has shown a difference.
  `ftCo_800C7CA0`'s wall-bounce star (`ftKb_SpecialN_800F1F1C`,
  `CharacterCallbacks::WALL_BOUNCE_EFFECT`) is ported from the C without
  a witness: no scene has bounced a prone Kirby off a wall.
- **Midair jumps** are the shared `ftCo_JumpAerialF1.c` rows Jigglypuff
  uses; `ftKb_DatAttrs` starts with the same `Fighter_x2D0_t` block. With
  a hat the helmeted rows 346..350 play instead (`ftCo_800D7268` ->
  `ftKb_SpecialN_800F1CD8`): `multi_jump_family` fails closed there.
- **The dash attack** has its own rows (351 on the floor, 352 in the air).
  `ftCo_AttackDash.c`'s decideFighter becomes
  `CharacterCallbacks::DASH_ATTACK_ROW`: the common entry on that row. Off
  a ledge it continues in the air at the same frame and rate (flags
  0x0C4C5082), with TransN's forward delta as horizontal speed and the
  fighter's own gravity, and lands back into 351; the dash-grab window
  (`mv.co.attackdash`) survives both changes.
- **The rapid jab** is the common Attack100 with a script effect of its
  own: efAlt 0x49D, one fist model per punch (`melee-ef` `kirby.rs`).
- **Throws.** The down throw is the common grounded throw. The forward,
  back and up throws leave the ground from their scripts
  (`ftCo_ThrowF_Phys` / `ftCo_ThrowF_Coll`'s airborne arms, the freeze on
  the script's flag, the landing that resumes it) and are not ported;
  `CharacterCallbacks::THROW_ENTRY` names them. In the forward and back
  throws the victim keeps its grab timer and can mash out
  (`ftCo_800DE3FC`); the up throw sets `x2219_b2` on both fighters (no
  blast-zone test) and uses `ftCamera_800762F4`.
- **Landing** clears the three once-per-flight lifts (`x64` Hammer, `xC4`
  copied Ice Shot, `xCC` copied Toad).
- **Items.** `ftKb_Init_OnItemPickup` maps hold kinds to hand poses
  differently from `Fighter_OnItemPickup` (1 -> 0, 3 -> 1, 4 -> 2) and is
  not ported (`CommonBehavior::item_hand` is `None` for Kirby).

## Not ported (fail closed)

| What | Where it stops |
| --- | --- |
| Inhale and its 30 rows, swallow, spit, the star (ftkirbyspecialn.c) | `enter_special` Neutral |
| Hammer (ftkirbyspecials.c), Final Cutter (ftkirbyspecialhi.c), Stone (ftkirbyspeciallw.c) and their articles | `enter_special` Side / Up / Down |
| Every copied neutral special (ftkirbyspecial<kind>.c), hats, the helmeted multijumps, losing the ability | rows 399..543; `ENTER_TAUNT` and `multi_jump_family` with a hat |
| Forward, back and up throws | `THROW_ENTRY` |
| CaptureKirby / CaptureWaitKirby / ThrownKirby on the victim | common rows, unported |
| Item pickup hand poses | `item_hand` |

## Evidence

- `start_fd_kirby_fox4` and `_cold`: 600 ticks, 0 differing particle ticks
  (`m4_gate`).
- `KIRBY_WITNESSES` (`m5_gate`): twenty-one scenes, all recorded once and
  exact on the first gate: the thirteen movement scenes, Kirby's down
  throw with a pummel, Fox's four throws on Kirby, Fox's up smash, dash
  attack and Reflector on Kirby through a tumble, the floor, a hit while
  lying down, a fall past the blast zone and the revival (whose Stone-form
  draw the seed checks), and the rapid jab whiffed and on Fox.
- Slippi: the Kirby games stop at the first special or airborne throw
  (see the stage report for frames matched).
