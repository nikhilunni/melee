# Kirby

Kirby (PlKb.dat, scenario name `Kirby`) vs Fox on Final Destination,
boundary `start_fd_kirby_fox4`. Crate: `ft-kirby`. Data facts are in
`docs/KIRBY_DATA.md`.

**Status: stage 1 of three.** Kirby is registered and his movement, common
attacks, down throw, hit reactions, death and revival are exact. His own
specials are stage 2: the Inhale that catches nothing and Final Cutter with
its beam are exact; the Inhale's catch, Hammer and Stone are not ported.
The copy abilities (stage 3) are not ported. Every unported row and entry
fails closed, as do his three airborne throws.

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
- **Final Cutter** (`ft-kirby/src/special_hi.rs`, `it-kirby`). Six of the
  table's eight rows are live: the start (385, 389 in the air), the rise
  (390), the fall (391) and the landing (388, 392 off the floor's end).
  Nothing enters 386 or 387: every motion change in ftkirbyspecialhi.c
  goes to one of the other six, so they stay unported rows.
  - Both starts draw the blade (efAlt 0x494: three models, two on joint 44
    and one on joint 1, `fp->parts` read by raw index). Their update
    callback (`efLib_Cb_SetRotYAndTransition`) replays frames 60..65 once
    the 0x41 frames are up, and plays out from 65 once the landing marks
    Kirby's effects to expire (`efLib_SetFlags`, `EffectRequest::ExpireOwned`).
    Two of the three turn with Kirby: the callback reads the fighter's
    facing through `HSD_JObj.scale.x` (Fighter +2C), which the port sends
    as `EffectRequest::OwnedFacing` when the stick turns him.
  - The grounded start's physics calls ft_80084FA8 and then replaces its
    acceleration with the stick's (ftCommon_8007CADC, no friction): a level
    stick stops Kirby on the first tick. The aerial start and the rise take
    their velocity from TransN (ft_80085134), the upward part scaled by
    +D4; the fall has no gravity and keeps the rise's last velocity.
  - The rise cannot land or catch a ledge for its first 21 collision
    passes (`mv.kb.specialhi.x0 > 0x14`); it spends every jump on each of
    them (ftCommon_8007D60C).
  - The landing rows carry accessory4 `fn_800F21E8`: on the script's
    cmd_vars[2], once per move, the beam leaves from TopN (+DC ahead, +E0
    up, one unit down) with Kirby's root X rotation as its slope. The
    landing tips that root with the floor.
  - The beam starts grounded and follows the floor's angle (it_8026D564,
    new in `melee-it`); past the floor's end it lifts 0.001 and flies level.
    A hit does not end it; its hitlag holds its lifetime, so it can hit a
    fighter twice. A reflection keeps the lifetime it has left.
  - The collision callbacks' changes install `ftKb_Init_800EE7B8` /
    `ftKb_Init_800EE74C` (the entries do not): a hit in the landing levels
    the root and destroys Kirby's effects.
  - **A shared fault this found:** a motion change that starts mid-animation
    without `Ft_MF_UpdateCmd` (the 385 -> 389 change, flags 0xC1082) left
    the new script's timer at 0, so its commands ran late by the start
    frame. Retail sets the timer to `-anim_start` for every such entry
    (fighter.c:1287) before ftAction_80073354 seeks; `melee-ft` now does.
- **Landing** clears the three once-per-flight lifts (`x64` Hammer, `xC4`
  copied Ice Shot, `xCC` copied Toad).
- **Items.** `ftKb_Init_OnItemPickup` maps hold kinds to hand poses
  differently from `Fighter_OnItemPickup` (1 -> 0, 3 -> 1, 4 -> 2) and is
  not ported (`CommonBehavior::item_hand` is `None` for Kirby).

## Not ported (fail closed)

| What | Where it stops |
| --- | --- |
| Inhale and its 30 rows, swallow, spit, the star (ftkirbyspecialn.c) | `enter_special` Neutral |
| Hammer (ftkirbyspecials.c), Stone (ftkirbyspeciallw.c) and their articles | `enter_special` Side / Down |
| Final Cutter's aerial start landing (ftkirbyspecialhi.c:447-453) and its aerial landing row landing (:526-536): neither row descends on its own | `air_start_collision`, `air_landing_collision` |
| The beam on a slope (itkirbycutterbeam.c:93-97, the one `fmadds`), landing from the air (:128-133), ended by a wall (:137-151), clanked (:162), absorbed (:167), glancing off a shield (:205-243) | `it-kirby` `cutter_beam.rs` |
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
- Final Cutter: fifteen `kirby_cutter_*` scenes in `KIRBY_WITNESSES`, items
  compared, all exact with 0 differing particle ticks: from the ground and
  the air, turned round, at and near the left edge (the beam off the
  floor's end, the start sliding off, the fall to the blast zone, Fox
  pushing the landing off), the ledge from the rise and from the fall, the
  beam and the sword on Fox, his shield, his Reflector on the floor and in
  the air, and Fox striking Kirby out of the start and the landing. The
  unreached branches were replaced by `unimplemented!` and the fifteen
  gated again, so none of them is taken. Searched and not reached on Final
  Destination: a clank (Fox's forward and down smash into the beam, eleven
  timings each: the beam hits him every time) and a reflecting shield (the
  beam is grounded wherever Fox can shield, and only an airborne item
  glances off).
- Slippi (`public-v3.7-b`: the four games in `KIRBY` and one in
  `GAMEANDWATCH`, all refused before Kirby was registered): every frame
  matches up to the first unported boundary, which the runner names.

  | Game | Frames matched | Stops at |
  | --- | ---: | --- |
  | `14_16_24 Marth + Kirby (PS)` | 2488 of 7001 (1378 before Final Cutter) | Kirby's back throw |
  | `HNC 5 ... Captain Falcon vs LOVE Kirby` | 1671 of 13271 | Kirby's up throw |
  | `Falco vs Kirby [YS]` | 1756 of 9109 | Kirby's forward throw |
  | `Game_20190824T180214` (Kirby against Kirby) | 1175 of 13809 | the Inhale catching a fighter |
  | `18_09_37 Kirby + Mr. Game _ Watch (FoD)` | 563 of 15399 | Kirby's forward or back throw |

  `replay-batch` before and after on both corpora: 106 of 108 and 396 of
  502 complete either way; those five games are the only changes.

- The explorer was not run from this boundary: its policies press B within
  the first seconds, so every case would stop at a special's entry.
