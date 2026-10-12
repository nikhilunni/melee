# Kirby

Kirby (PlKb.dat, scenario name `Kirby`) vs Fox on Final Destination,
boundary `start_fd_kirby_fox4`. Crate: `ft-kirby`. Data facts are in
`docs/KIRBY_DATA.md`.

**Status: stage 2 of three, in part.** Kirby is registered and his movement,
common attacks, down throw, hit reactions, death and revival are exact. Of
his own specials (stage 2) the Inhale, with a caught fighter held, spat and
swallowed, and Final Cutter with its beam are exact; Hammer and Stone are not
ported, nor is inhaling an item. A swallow leaves the copied kind as state;
the copy abilities themselves (stage 3) are not ported. Every unported row
and entry fails closed, as do his three airborne throws.

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
- **The Inhale's catch** (`ft-kirby/src/mouthful.rs`, `swallow.rs`; the
  caught fighter's states are common code, `melee-ft` `capture_kirby.rs`,
  driven between the two fighters by `melee-lib` `grab_pairs.rs` like the
  other capture families). Rows 356, 358..367, 369 and 374, 376..378, 380,
  382 are live; the six item rows (357, 368, 370, 375, 379, 381) are not.
  The reach is about 22 units: Fox at 24.4 is not caught, at 21.6 he is.
  - The grab is the special grab armed since the whiff (category 0x10).
    Kirby's grab_cb (fn_800F6210 / fn_800F6318) takes SpecialNCapture0;
    the victim's grabbed_cb (ftCo_800BD1DC) turns it to face him in
    CaptureKirby, still vulnerable, and its accessory1 (ftCo_800BD39C) moves
    it each tick at most 0.9 across and 1.2 up toward the mouth (5.4 ahead
    of Kirby, 3.4 up), shrinking it inside 7.4 units to 55% at the mouth.
    Kirby's own Anim tests the squared distance (under 1.0) and closes the
    mouth: ftCo_800BD620 pins the victim to his TransN2 (the thrown
    fighters' constraint), hidden and intangible at its own size, with no
    animation; Kirby gulps (Eat) and stands (EatWait).
  - The victim's IASA (ftCo_CaptureWaitKirby_IASA) reaches into Kirby, so
    the scene runs it on the pair: while Kirby is in EatWait or EatFall a
    tap up hops him off the floor into EatFall at 0.6 (ftKb_SpecialS_800F6BB0,
    `CharacterCallbacks::MOUTH_HOLD`) and a tap sideways sets his speed to
    0.6 that way; each tap is spent (its tilt timer becomes 254). The hold
    timer starts at 250, loses 1 a tick and 12 per mash input
    (ftCommon_GrabMash); at its end the victim is set down facing away
    (ftCo_800DC920, CaptureCut) and Kirby lets go (CatchCut).
  - With a mouthful Kirby walks (ftWalkCommon on rows 360..362 with the
    multiplier 0.6 and the common walk rates), turns (363), jumps (364 is
    the squat with the common short-hop test, 365 the jump with
    ftCo_800CB110 scaled by 0.6; 365 has no IASA, so nothing can be done in
    it), lands (366) and falls (377, entered off a floor's end, from the
    aerial gulp or from the victim's hop; the only aerial row that takes
    input). Every row has its ground/air counterpart at the same frame.
  - A (EatWait or EatFall) spits: on the script's flag ftCo_800DE2CC sets the
    victim free at TransN2 with the script's throw record logged as damage
    alone (10% on Fox; a swallow's is 8%), and ftCo_800BDB58 makes it a star (ThrownKirbyStar, playing
    Kirby's submotion 285 and its script): hidden, intangible, flying level
    at 4.0 less 0.13 a tick, its facing taken from the vertical speed's sign
    each tick (retail 0x800BDE8C), with a hitbox of the victim's own co_attrs
    +E0 that answers to Kirby. Its timer is 30, less 1 a tick and 12 per
    mash input. At its end, or when what its box meets turns it back
    (ftCo_ThrownKirbyStar_Coll: lbVector_Mirror, the dot product an
    `fmadds`), the victim comes out (ThrownKirby) at 0.73 across and 2.3 up,
    visible and vulnerable, growing back over 4 ticks (accessory4
    ftCo_800BE6AC), then falls.
  - B or the stick down past 0.4 swallows: the same release into the copy
    star (ThrownCopyStar, submotion 284, no hitbox), 2.8 at 75 degrees less
    0.17 a tick for 10 ticks; out of it the victim falls at once. Twelve
    per mash input is more than that timer: one press ends it on its first
    tick (the Slippi game below does exactly that).
  - **The copied ability is state only** (`Kirby::copied`,
    ftKb_SpecialN_800F1BAC). ftCo_800BD9E0's kind arms are
    `CharacterCallbacks::SWALLOWED_ABILITY` (Kirby gives his own hat's
    kind; a Kirby with one is refused, ftCo_800BDA50). Most hats are a
    model of their own, loaded by ftKb_LoadHat and drawn at Kirby's part 6
    matrix (ftKb_UnkMtxFunc0): nothing the simulation reads. Five kinds
    (Donkey Kong, Jigglypuff, Mewtwo, Falco, Mr. Game & Watch: the LOAD_HAT
    loaders, with replacement models) graft parts into his skeleton and
    fail closed at the swallow. With an ability, a hit draws
    `HSD_Randi(32)` for its loss in Fighter_ProcessHit (fighter.c:2860 and
    2960, `CharacterCallbacks::DAMAGE_DRAW`): the draw is ported, taking the
    ability away is not. The midair jumps are the bare rows under every hat
    but five (fn_800F1CA0: Captain Falcon, Sheik, Popo, Nana, Samus), which
    fail closed. Death takes the ability (ftKb_Init_800EE74C).
  - **Shared code this added to:** a fighter that dies pinned to its captor
    (in a falling Kirby's mouth it crosses the blast zone first) is set free
    by ftCo_800DD100 before its death entry; the port does it right after,
    which is the same while the release does not move it, and refuses
    otherwise (`release_from_dead_partner`). ftCo_800DC920 has a Kirby arm
    for a pair parted during the pull (ftCo_CaptureCut.c:115-122), which
    fails closed.
- **Landing** clears the three once-per-flight lifts (`x64` Hammer, `xC4`
  copied Ice Shot, `xCC` copied Toad).
- **Items.** `ftKb_Init_OnItemPickup` maps hold kinds to hand poses
  differently from `Fighter_OnItemPickup` (1 -> 0, 3 -> 1, 4 -> 2) and is
  not ported (`CommonBehavior::item_hand` is `None` for Kirby).

## Not ported (fail closed)

| What | Where it stops |
| --- | --- |
| Inhaling an item (fn_800F6178 / fn_800F6280, ftKb_SpecialN_800F5DE8, it_802F23EC) and its six rows | nothing arms an item's catch; rows 357, 368, 370, 375, 379, 381 |
| The held fighter dropping Kirby through a platform (ftCo_CaptureWaitKirby.c:56-62) | `capture_kirby` `steer_vertically` |
| The pull and the gulp sliding off a floor's end (fn_800F697C, fn_800F6AC8: ftkirbyspecialn.c:565-571, 591-598); a mouthful's wait with no victim (:1211-1243) | `mouthful` `capture_collision`, `eat_collision`, `swallow_or_spit` |
| A pair parted while the Inhale still pulls its catch in (ftCo_CaptureCut.c:115-122) | `refuse_parting_in_the_pull` |
| A fighter out of Kirby's mouth landed by its release (ftCo_CaptureCut.c:29-30); one dying in his mouth moved by it (ftCo_CaptureCut.c:137-189) | `capture_kirby` `escape`, `release_from_dead_partner` |
| A star on a floor or under a ceiling (ftCo_ThrownKirby.c:76-77, 82-83), slower than its deceleration (:177-179), and a hit on the fighter out of one (:340-343) | `capture_kirby` `contact_normal`, `star_physics`, `star_hit_taken` |
| A mouthful walked over a slippery floor or with no forward speed (ftwalkcommon.c:106-112); a take-off past the scaled speed limit (ftCo_Jump.c:133-139) | `mouthful` `walk_anim`, `take_off` |
| A hat grafted into Kirby's parts (ftkirby.c:3578-3592: Donkey Kong, Jigglypuff, Mewtwo, Falco, Mr. Game & Watch) | `swallow` `copy_ability` |
| Losing the ability to a hit (ftkirbyspecialn.c:266-291, the star it_802ADA1C) or a taunt; a Kirby with an ability inhaled (ftCo_ThrownKirby.c:63-69, 283-289) | `ability_loss_draw`, `ENTER_TAUNT`, `SWALLOWED_ABILITY` |
| Hammer (ftkirbyspecials.c), Stone (ftkirbyspeciallw.c) and their articles | `enter_special` Side / Down |
| Final Cutter's aerial start landing (ftkirbyspecialhi.c:447-453) and its aerial landing row landing (:526-536): neither row descends on its own | `air_start_collision`, `air_landing_collision` |
| The beam on a slope (itkirbycutterbeam.c:93-97, the one `fmadds`), landing from the air (:128-133), ended by a wall (:137-151), clanked (:162), absorbed (:167), glancing off a shield (:205-243) | `it-kirby` `cutter_beam.rs` |
| Every copied neutral special (ftkirbyspecial<kind>.c), the hat models, the helmeted multijumps | rows 399..543 (`enter_special` Neutral with an ability); `multi_jump_family` under the five helmets |
| Forward, back and up throws | `THROW_ENTRY` |
| Item pickup hand poses | `item_hand` |

## Evidence

- `start_fd_kirby_fox4` and `_cold`: 600 ticks, 0 differing particle ticks
  (`m4_gate`).
- `KIRBY_WITNESSES` (`m5_gate`) holds sixty-seven scenes: the twenty-one
  below, the Inhale's whiff, Final Cutter's fifteen and the catch's
  thirty. The twenty-one were all recorded once and
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
- The Inhale's catch: thirty scenes in `KIRBY_WITNESSES`, each exact over
  its full length with 0 differing particle ticks. Thirty-five were
  recorded for it, each once. Five (`kirby_inhale_{spit,swallow,hold,mash,
  struggle}_fd_fox4`) stopped the run-in eleven ticks early, at x=35.6,
  just out of the Inhale's reach: they are whiffs (exact, but not evidence
  of a catch) and are not registered; their scenario files and traces are
  still in the main checkout. Of the thirty, twenty-nine were exact on
  their first gate and `kirby_mouthful_edge_death` on its second: the first
  stopped at the shared death code's refusal of a pinned fighter, ported
  then. The first five scenes' inputs were written before the port existed;
  the other twenty-five were timed on the port's own dry runs to reach
  their states (several were retimed there before being recorded), and no
  input was changed after a recording. Six headers were rewritten after
  recording to say what retail did.
  - What they cover: the catch out of the start and out of the loop, on the
    floor and in the air; the pull (also landing during it), the gulp
    (also ending in the air) and the wait; A and the spat star (thirty
    ticks along the floor and past the edge, toward the middle and landing,
    turned back at once by the stage's side, glancing down off its slant);
    B and the stick down, the copy star; the hold timer on its own, with A
    mashed, and ending while Kirby is in the air; the held fighter's taps
    up and sideways, on the floor and in the air, one pushing Kirby off the
    floor's end; the three walks, the turn, the jump on X and on the stick,
    short and full, out of the wait and out of the walk, the landing;
    walking off an edge into EatFall, the aerial turn, spit and swallow
    there; every other row's ground/air counterpart (the spit, the swallow
    and the turn landing and leaving the floor, the squat leaving it); both
    falling to the blast zone, the held fighter first; and Kirby with Fox's
    ability hit twice with knockback and three times without (five draws),
    jumping, on a ledge and dying.
  - How the rest was found: the retail histories' state changes against
    every change the code can make, and a scratch run with a marker in each
    single-line `fn`, `if`, `else` and match block of the three new files
    (it cannot see a block opened by a multi-line condition, nor the code
    after an early return, which is where the counterparts are: those came
    from the histories). What neither reached is a refusal now
    (`unimplemented!`): the rows of the table above from the platform drop
    to the mouthful's take-off.
  - Live without a scene of its own: a swallow that gives the kind Kirby
    already has (only a Kirby swallowing a Kirby; the Slippi game below
    does it and matches); the star's hitbox damage (the star answers to
    Kirby, and the scenes have no third fighter for it to hit); any kind's
    ability but Fox's; EatLanding leaving the floor (the callback the walk
    and the squat share, fn_800F6450); and the one-in-32 loss itself, which
    none of the five draws came to.
  - Other captures, gated before and after: the 207 older `m5_gate` scenes
    whose names mention a grab, throw, catch, capture, pummel, Egg Lay,
    Klaw, Confusion, cargo or Falcon Dive (201 of other characters, six of
    Kirby's own grab and throws) are all exact with this change, as they
    were in the sweep of the commit before it (db6f8b1f).
- Slippi (`public-v3.7-b`: the four games in `KIRBY` and one in
  `GAMEANDWATCH`, all refused before Kirby was registered): every frame
  matches up to the first unported boundary, which the runner names.

  | Game | Frames matched | Stops at |
  | --- | ---: | --- |
  | `14_16_24 Marth + Kirby (PS)` | 2488 of 7001 (1378 before Final Cutter) | Kirby's back throw |
  | `HNC 5 ... Captain Falcon vs LOVE Kirby` | 1671 of 13271 | Kirby's up throw |
  | `Falco vs Kirby [YS]` | 1756 of 9109 | Kirby's forward throw |
  | `Game_20190824T180214` (Kirby against Kirby) | 5426 of 13809 (1175 before the catch) | Kirby's up throw |
  | `18_09_37 Kirby + Mr. Game _ Watch (FoD)` | 563 of 15399 | Kirby's forward or back throw |

  `replay-batch` before and after on both corpora: 106 of 108 and 396 of
  502 complete either way; those five games are the only changes. (That
  comparison predates the catch; the Kirby against Kirby row is from the
  four-game run alone.) In that game one Kirby catches the other at tick
  1176, turns and spits him (the star mashed out of after 17 ticks), and at
  3480 is caught in turn and swallowed, mashing out of the copy star on its
  first tick.

- The explorer was not run from this boundary: its policies press B within
  the first seconds, so every case would stop at a special's entry.
