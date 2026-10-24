# Kirby

Kirby (PlKb.dat, scenario name `Kirby`) vs Fox on Final Destination,
boundary `start_fd_kirby_fox4`, and on Yoshi's Story, boundary
`start_ys_kirby_fox4` (for Stone on a slope). Crate: `ft-kirby`. Data facts
are in `docs/KIRBY_DATA.md`.

**Status: stage 2 of three.** Kirby is registered and his movement, common
attacks, four throws, hit reactions, death and revival are exact. His own
four specials (stage 2) are exact: the Inhale, with a caught fighter held,
spat and swallowed; Hammer; Final Cutter with its beam; and Stone. Inhaling
an item is not ported. A swallow leaves the copied kind as state; the copy
abilities themselves (stage 3) are not ported. Every unported row and entry
fails closed.

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
  back and up throws leave the ground from their scripts, on common code
  no other kind's script reaches (`melee-ft` `throw_flight.rs`; the pair is
  driven by `melee-lib` `grab_pairs.rs` `thrown_request` /
  `thrown_escape`). Kirby's own facts are `CharacterCallbacks::THROW_TRAITS`.
  - The script's SetAirborne (frame 9 forward, 17 back, 6 up) lifts Kirby
    with every jump spent; in the air his velocity is TransN's
    (`ftCo_ThrowF_Phys`, ft_80085134) and his collision never lands him by
    itself (`ftCo_ThrowF_Coll`).
  - On the script's flag (cmd var 0: frame 33 forward, 23 back, 45 up)
    `ftCo_800DD724` stops his animation and keeps TransN's offset of that
    frame as his velocity (`mv.co.throw`), so he falls at a constant speed
    for as long as it takes; `ftCo_800DE920` stops the held fighter on the
    same frame (at once if it is already there, which is the case when it
    comes before Kirby in the fighter list; else when its own animation
    arrives).
  - A floor under the frozen Kirby (ft_80083C00, `fn_800DD568`) runs his
    animation one step and his animation callback inside the collision
    callback, resumes the held fighter with a step of its own
    (`ftCo_800DE974`) and stands him. The throw plays out on the floor: the
    back throw releases there; the forward and up throws lift him again on
    the release's frame, and that second flight cannot land
    (`mv.co.throw.x8`, ft_80083B68), so they end in Fall.
  - Over a pit nothing ends the freeze: both fall to the blast zone (the
    forward throw travels 17 units, the back throw 7 the other way).
  - In the forward and back throws the held fighter keeps its grab timer
    (`ftCo_800DE3FC`; 76 ticks from the grab in these scenes, at 0%), which
    loses 1 a tick and 18 per mash input in the throw (PlCo +3C8; 6 in the
    hold before it). When it ends, `ftCo_800DE5A4` takes CaptureJump (the
    stick up, or a jump latched in CaptureWait) or CaptureCut;
    ftCo_800DC920 sets the fighter down where it hangs and Kirby goes on
    with nobody, frozen until he lands if he was frozen. The up and down
    throws take the timer away.
  - The up throw rises to y=249 on Final Destination: `x2219_b2` on both
    fighters (`Status::blast_zone_exempt`) keeps ftCo_800D3158 off them
    until the flag, and neither camera box follows
    (`ftCo_ThrowHi_Cam` / `ftCo_ThrownHi_Cam`: ftCamera_800762F4 moves the
    camera bone alone). After its release the last hop goes 18 units back.
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
    loaders) hang their DObjs on his thirteen empty parts instead, drawn
    only as well (`crate::hat`). What the trace does see is the callbacks
    the swallow installs: death1_cb / death3_cb (ftKb_Init_800EE7B8 /
    800EE74C), which unlike the specials' take_dmg_cb / death2_cb survive
    motion changes, so the first hit after a swallow destroys Kirby's
    effects (and parks the particle insertion cursor), and the copied kind's
    put-away arms (ftKb_SpecialN_800F19AC on a loss or death, 800F1A8C on
    that hit): Donkey Kong, Samus and Mewtwo destroy Kirby's effects again,
    Mewtwo also clears the secondary color slot. With an ability, a hit draws
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
- **Hammer** (`ft-kirby/src/special_s.rs`, `it-kirby/src/hammer.rs`). One
  row on the ground (383) and one in the air (384), with no change between
  them.
  - A dash into Hammer keeps almost none of its speed (1.4 becomes 0.2 on
    the entry's frame) and stops within half a unit: the grounded swing
    does not leave a floor's end on its own.
  - The first aerial Hammer of a flight sets the upward speed to +CC (1.5)
    and spends the lift (`u.kb.x64`) until a landing; the second does not
    rise. An aerial swing that reaches the floor lands in
    LandingFallSpecial with +D0's lag (16 frames); a grounded swing that
    loses the floor (Fox pushed it off) ends in Fall.
  - The hammer is an article. accessory4 (fn_800F53AC) creates it in
    `fp->parts[44]` on the script's cmd_vars[0] = 1, which is the first
    frame, and destroys it on 2. It has one state without callbacks: the
    hitboxes are Kirby's own. With it come the trail on the hip (efAlt
    0x496, or 0x497 in the air), the article's own model on its attach joint
    (efAlt 0x498 / 0x499, by the owner's ground state at the swing), and
    the damage and death callbacks, which remove it
    (ftKb_SpecialAirLw_800F5524).
  - A side special with the stick the other way turns Kirby round first
    (the common entry).
- **Stone** (`ft-kirby/src/special_lw.rs`). Three rows on the ground (393
  the change, 394 the rock, 395 the change back) and three in the air
  (396..398).
  - The start's script asks for the rock (cmd_vars[0] = 1): the shell takes
    +108's health, the form becomes `(form + HSD_Randi(4)) % 5 + 1` (so
    never the form before; the first follows the reset's draw), and the
    model flickers into the rock over 22 frames by the table
    ftKb_Init_803CB490 (ftParts_80074B0C: group 0 the form's model, group 1
    hidden; the held item, the articles and the hat hidden). The end's
    script asks for the way back (2), the same table backward. The draw
    follows the script's own graphics of that frame.
  - The rock row holds its animation (rate 0). Each frame its animation
    callback sets `x221C_b4`; the start and end rows clear it.
  - **The shell is shared code** (`melee-ft` `CombatState::through_shell`):
    while `x221C_b4` is set, a hit's damage comes off `dmg.x1834` before it
    is logged. While the shell holds nothing is logged: no damage, no
    hitlag for Kirby, no reaction; the attacker still has its hitlag and
    its stale-move credit, and the spark is the invincible one (1052 /
    0x41C). The hit that takes the health below zero is logged with what
    is left over, and clears the bit.
  - A shell broken by a hit that does not move Kirby (a Blaster shot) is up
    again on the next frame, since the rock row sets the bit every frame,
    with its health below zero: each later hit then does its own damage
    plus all that the health is below zero. Retail does this
    (`kirby_stone_laser_break_fd_fox4`: 2.11%, then 4.6% more).
  - Any motion change clears the bit (fighter.c:1058), so a grab takes the
    shell and, with no SkipModel flag, the rock's model: Fox grabs and
    throws the rock as he would Kirby. Stone's own changes put the bit back
    after each `Fighter_ChangeMotionState`.
  - The rock ends when its timer (150 frames, counted in the IASA callback)
    is spent, or on a B press once 18 of them have passed; an earlier press
    is lost. The grounded end leaves the floor (ftCommon_8007D5D4): both
    end rows are airborne, move on their TransN (which only rises, 8.2
    units) and finish in Fall.
  - In the air the start hovers (its TransN dips 0.77 on the first frame and
    then rises: started within that of the floor it lands at once into the
    grounded start), and the rock falls at +104 (4.5 a frame). Its landing asks for the large
    quake and continues in the grounded row; a grounded rock or start that
    loses its floor continues in the aerial one with no horizontal speed.
    The falling rock hits Fox; a rock formed on the floor did not hit him
    standing against it.
  - Fox can push the rock. On a floor past +F4's angle (ten degrees) it
    slides: each frame the collision callback reads the floor's normal and
    its angle (800F3570), keeps the last four (800F37EC), and the physics
    callback lays the speed along their normalised sum (800F3B28), gaining
    +FC times the normal's x a frame. A rock that lands on such a floor
    starts at +100 times the normal's x. Once a frame newer than a sliding
    one is inside the angle again, the speed and acceleration take a factor
    0.9 and the rock brakes by the fighter's ground friction.
  - The damage and death callbacks are installed by the entry, by the
    collision callbacks' changes and by the end, but not by the start's
    change into the rock (ftKb_SpecialLw1_Anim): a grounded rock that never
    left the floor has none. They would only clear what the motion change
    after the hit clears anyway.
  - Stone's scripts use subaction opcode 53 (x2225_b2, the hat's
    visibility), new in `melee-cmd`.
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
| Stone's rock where the floor ahead slopes back (ftkirbyspeciallw.c:177-188), a slope that turned against the slide within four frames (:273-285), a floor past fifty degrees (:308-313), the slide at its top speed (:338-352) | `special_lw` `read_floor`, `record_floor`, `slide` |
| Stone's end rows touching a floor (ftkirbyspeciallw.c:695, 866: the result is ignored); the special fall after Stone with a nonzero +114 (:471, 510) | `special_lw` `end_collision`, `end_anim` |
| A phantom hit's damage against the shell (ftColl_8007BE3C, ftcoll.c:3666-3676) | `melee-ft` `expire_phantom_lockout` |
| Hammer or Stone with an item in hand (ftCommon_8007F5CC) | `melee-ft` `require_held_item_state` refuses a special row with a held item; `special_lw` `show_held_item` asserts it |
| Final Cutter's aerial start landing (ftkirbyspecialhi.c:447-453) and its aerial landing row landing (:526-536): neither row descends on its own | `air_start_collision`, `air_landing_collision` |
| The beam on a slope (itkirbycutterbeam.c:93-97, the one `fmadds`), landing from the air (:128-133), ended by a wall (:137-151), clanked (:162), absorbed (:167), glancing off a shield (:205-243) | `it-kirby` `cutter_beam.rs` |
| Every copied neutral special but the Blaster and the Falcon / Warlock Punch (ftkirbyspecial<kind>.c), the hat models, the helmeted multijumps | rows 399..543 (`enter_special` Neutral with an ability); `multi_jump_family` under the five helmets |
| An airborne throw landing before its script's flag (fn_800DD5EC, ftCo_Throw.c:207-223: the animation run forward to the flag); the release or the throw's end inside the callback a landing runs (ftCo_Throw.c:196-198, 213-215) | `throw_flight` `land_throw_before_flag`, `run_throw_callback_in_landing` |
| A throwing pair losing its floor (fn_800DD684, ftCo_Throw.c:225-234); a kept grab timer out of a state that is not a capture wait (ftCo_Thrown.c:36) | `callbacks::collision::grab_hold`, `ThrownHold::entering` |
| Item pickup hand poses | `item_hand` |

## Evidence

- `start_fd_kirby_fox4` and `_cold`: 600 ticks, 0 differing particle ticks
  (`m4_gate`).
- `start_ys_kirby_fox4` and `_cold`: 600 ticks, 0 differing particle ticks
  (`m4_gate`); made for Stone's slopes with `make_boundary.py`, exact on its
  first gate.
- `KIRBY_WITNESSES` (`m5_gate`) holds 116 scenes: the twenty-one below, the
  Inhale's whiff, Final Cutter's fifteen, the catch's thirty, the throws'
  twenty-three, Hammer's eight and Stone's eighteen. The twenty-one were all recorded once and
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
- The forward, back and up throws: twenty-three `kirby_throw_*` scenes in
  `KIRBY_WITNESSES`, each recorded once and exact on its first gate over
  its full length with 0 differing particle ticks. The three mid-stage
  scenes were recorded before the port existed; the rest were timed on the
  port's dry runs, and no input was changed after a recording. Six headers
  were rewritten afterwards to say what retail did.
  - What they cover: each throw in mid-stage; the forward throw whole and
    with Fox's timer ending on the floor before the release; flights that
    end just inside an edge (forward at the right, back at the left) and
    past it, both fighters falling frozen to the blast zone (forward and
    back, off either edge, facing either way); Fox mashing out of the
    forward and back throws by CaptureCut and CaptureJump, on the floor
    before the flight, in the flight, and out of the frozen fall past the
    edge (Kirby falling on with nobody); a jump latched in CaptureWait; the
    up throw beside an edge (its last hop ends past it: Fall and the
    ledge), twice in a row, and with Fox mashing in it and in the down
    throw (nothing happens: the timer is gone).
  - The blast-zone exemption is seen through Fox: without his, the port
    kills him at the top of the up throw (he is on the floor's side of
    ftCo_800D3158's test); Kirby, airborne without knockback, would pass
    the top either way, so his own bit cannot be told apart.
  - The camera boxes are not in the gate's keys: with the two `_Cam`
    callbacks following their fighters every scene still gates exact.
    `kirby_throw_hi_camera_fd_fox4` was recorded with `record.py --camera`
    and both fighters' subjects (position, camera bone, facing, target
    extents) compared with the port's over its 420 ticks by a scratch
    extension of `camera-diff` (not committed): 0 differing rows, and 118
    with the Kirby arms disabled.
  - Kirby is the first fighter in every scene, so the held fighter is one
    animation step behind when he freezes. The other order (the held
    fighter already on the frame, ftCo_800DE920's first arm) is what the
    four Slippi games in `KIRBY` run: 33 freezes in the frames they match,
    none of them a stop. No retail trace has that order.
  - Not reached by any scene or by those games, and refused: a flight that
    touches a floor before its flag (fn_800DD5EC; no flight over level
    ground does, and its floor filter refuses platforms), and the release
    or the throw's end inside a landing's callback (no script has them on
    the frame after the flag).
  - Other kinds' throws, gated before and after: the 218 `m5_gate` scenes
    whose names mention a grab, throw, catch, capture, pummel, Egg Lay,
    Klaw, Confusion, cargo or Falcon Dive are exact with this change, as
    they were before it. No other kind's throw script sets cmd var 0 or
    leaves the floor.
- Hammer: eight `kirby_hammer_*` scenes in `KIRBY_WITNESSES`, items
  compared, each recorded once and exact on its first gate over its full
  length with 0 differing particle ticks. Two were recorded before the port
  existed; the rest were timed on the port's dry runs, and no input was
  changed after a recording. Two headers were rewritten afterwards to say
  what retail did.
  - What they cover: on the ground whiffed, facing either way, and on Fox;
    from a full hop with the lift and the landing in the swing; from a short
    hop on Fox (two hits); twice in one flight (the first ends in Fall, the
    second does not rise and lands); pushed off the floor's end by Fox in
    the grounded swing; struck by Fox's dash attack with the hammer out; and
    across the blast zone with it out, then off the revival platform.
  - Every branch of `special_s.rs` is taken by them (read off their motion
    and item histories; no marker build was run for Hammer).
- Stone: eighteen `kirby_stone_*` scenes in `KIRBY_WITNESSES`, items
  compared, each recorded once and exact over its full length with 0
  differing particle ticks. Two were recorded before the port existed
  (their inputs obvious); their first gate stopped at subaction opcode 53,
  unported then, and with it they were exact. The other sixteen were timed
  on the port's dry runs and exact on their first gate. No input was
  changed after a recording; one header was rewritten afterwards to say
  what retail did (the early B press that does nothing).
  - What they cover, on Final Destination: the whole 150 frames from the
    ground and from a full hop (the fall, the landing and its quake); a B
    press before the minimum time (nothing) and after it, on the floor and
    in the air; the aerial start landing on its first frame; five in a row
    (the forms follow the random stream); the rock past the blast zone, and
    Stone off the revival platform and on the floor after it (the form
    drawn at the revival, then past it); Fox's jabs on the shell, three up
    smashes breaking it, his Blaster's shots on it and breaking it after
    two up smashes (the fighter and the item pass of the shared hit code);
    his down smash before the rock has formed; his grab and throw of the
    rock; the falling rock on Fox; the rock and the start pushed off the
    floor's end by Fox.
  - On Yoshi's Story: the rock sliding down the left slope and off its end;
    a rock landing on that slope already sliding; and Fox pushing a rock on
    the right slope up onto the level floor, where it settles and brakes.
  - How the rest was found: a scratch build with a marker in each branch of
    `special_lw.rs` and of the shell's three sites, run over the eighteen
    scenes and the five Slippi games. What none reached is a refusal now
    (the first three rows of the table above that name Stone) and the
    eighteen were gated again. Not reached and still live: the brake of a
    rock moving left on a level floor (the mirror of the witnessed one).
  - The model's selections (the form's model, the hidden group, the hat's
    bit) are not in the gate's keys: `melee-lib`'s presentation test runs
    Stone and draws, and nothing compares the drawn rock with retail's.
  - Other kinds, gated with the shared changes (the shell in the fighter
    and item hit passes, opcode 53, x2225_b2 across motion changes): the 242
    `m5_gate` scenes whose names mention a grab, throw, catch, capture,
    pummel, Egg Lay, Klaw, Confusion, cargo or Falcon Dive are all exact.
- Slippi (`public-v3.7-b`: the four games in `KIRBY` and one in
  `GAMEANDWATCH`, all refused before Kirby was registered): every frame
  matches up to the first unported boundary, which the runner names.

  | Game | Frames matched | Stops at |
  | --- | ---: | --- |
  | `14_16_24 Marth + Kirby (PS)` | 7001 of 7001 (5884 before Hammer) | the end: every frame matches |
  | `HNC 5 ... Captain Falcon vs LOVE Kirby` | 13271 of 13271 | the end: every frame matches |
  | `Falco vs Kirby [YS]` | 2362 of 9109 | the Final Cutter beam ended by a wall |
  | `Game_20190824T180214` (Kirby against Kirby) | 13809 of 13809 (8704 before Hammer, 8847 before Stone) | the end: every frame matches |
  | `18_09_37 Kirby + Mr. Game _ Watch (FoD)` | 9909 of 15399 (5388 before the copied Chef, 5100 before the grafted hats) | Mr. Game & Watch's x in his aerial Fire (374) at FoD's left edge, Kirby beside him |

  The Kirby against Kirby game runs Stone 27 times (24 of them landing from
  the air), the Fountain of Dreams game three times before it stops. Neither
  has a hit on the rock or a slope under it.

  `replay-batch` before and after on both corpora: 106 of 108 and 396 of
  502 complete either way; those five games are the only changes. (That
  comparison predates the catch; the Kirby against Kirby row is from the
  four-game run alone.) In that game one Kirby catches the other at tick
  1176, turns and spits him (the star mashed out of after 17 ticks), and at
  3480 is caught in turn and swallowed, mashing out of the copy star on its
  first tick.

- The explorer has not been run from either boundary. It was held back
  while the specials refused (its policies press B within the first
  seconds); they no longer do, so it is the next check.

## Stage 3 so far (copy abilities)

- **Losing the ability** (`ftKb_SpecialN_800F5D04`): by taunt, by a hit whose
  `HSD_Randi` draw is 0 (`DAMAGE_DRAW`), by death. The ability star is an
  item (`It_Kind_Unk1`, `ftData.x48_items[2]`, `it-kirby` `ability_star.rs`);
  it clears xDCE b7, so its ECB does not turn with it
  (`ItemCore::ecb_follows_rotation`). The star against a wall or ceiling
  (`it_8026DFB0`, itgroundcoll.c:381-382) fails closed.
- **Helmeted midair jumps** (rows 346..350) for the hats of `fn_800F1CA0`
  (Captain Falcon, Sheik, Popo, Nana, Samus); witnessed with Falcon's
  (boundary `start_fd_kirby_captainfalcon4`).
- **Stale-move ids** 0x16..0x2F are `StaleMove::CopiedMario..CopiedGkoops`;
  no row uses them yet.
- **A wall ends the Cutter beam** (itkirbycutterbeam.c:144-151), witnessed on
  Yoshi's Story; the beam beside a wall with no horizontal speed (:138-143)
  fails closed.
- **The copied Falcon Punch and Warlock Punch** (ftkirbyspecialcaptain.c,
  `ft-kirby/src/copied/captain.rs`): Captain Falcon's states
  (`ft_captain_family::special_n`, generic over the `FalconPunch` trait
  that every `CaptainFamily` gets by a blanket impl) over rows 433/434
  (Falcon's hat) and 528/529 (any other, i.e. Ganondorf's), animations
  368/369 and 463/464.
  - One marker type (`KirbyPunch`): the entries and the two Coll callbacks
    pick the rows by `hat.kind != FTKIND_CAPTAIN`, and Phys picks the
    effect by the kind, so the trait's methods read Kirby's copied kind.
  - Both kinds read `specialn_ca_*` (+1F8..+208); `specialn_gn_*`
    (+20C..+21C) has no reader. Kirby's Anim callbacks push no wind
    (800F9AEC), unlike Ganondorf's own.
  - The punch's pair: efAlt 0x4AB (EfKbCa.dat, bank 38: model 0x9471 on
    `parts[0]`, then 0x9470 on `parts[44]`) and 0x4B3 (EfKbGn.dat, bank
    47: 0xB799, then 0xB798); both sync-loaded. The Warlock copy's
    particles 0xB798..0xB79A are efLib_SpawnParticleEffect's special case
    with Ganondorf's 0x4A38..0x4A3A (eflib.c:866-878: an AppSRT with the
    root's rot.y).
  - No articles, no scratch, no put-away arm (800F19AC / 800F1A8C have no
    case for either kind).
  - Not reached: the grounded punch losing its floor (ft_800827A0 stops it
    at an edge, so it needs a floor that goes away); live, shared with the
    originals' code.
- **The copied Shield Breaker and Flare Blade** (ftkirbyspecialmars.c,
  `ft-kirby` `copied/shield_breaker.rs`, rows 492..499 and 530..537): the
  states are ft-mars-family's `special_n`, generic over a `ShieldBreaker`
  trait (rows, attributes, release effect, charge flash, gusts) that every
  `MarsFamily` gets by a blanket impl. Kirby's differ from Marth's and
  Roy's in what the trait names: ftKb_DatAttrs +35C / +370 (the same five
  words as MarsAttributes +0), efAlt 0x4B4 / 0x4B5 (models 0x4E20 / 0x4E21,
  EfKbMs.dat, bank 20) and 0x4B8 / 0x4B9 (0xBB80 / 0xBB81, EfKbFe.dat, bank
  48) at TopN, the same flashes (99, 100), and no lb_800119DC gusts at the
  hips (retail 0x8010B958 has no call). No mv+4 word is retained (Kirby's
  other specials model none either), no take_dmg_cb is armed, and neither
  put-away nor 800F190C has a Marth or Roy arm. The entries also load the
  copy's sword (ftCommon_SetAccessory, hat_dynamics[0] on the right thumb,
  kept across the rows by Ft_MF_KeepAccessory 0x2000, which Kirby's motion
  changes add to Marth's flags): a model only, not in the port.
  `ftKb_Init_UnkMotionStates4` has no Marth or Roy arm. Witnesses: nine
  `kirby_copy_{shieldbreaker,flareblade}_*` scenes (`KIRBY_WITNESSES`).
- **The copied Rollout** (ftkirbyspecialpurin.c, rows 475..491, submotions
  410..426; `copied/rollout.rs`). Jigglypuff's states moved into the family
  crate `ft-purin-family`, generic over `RolloutKind`; Jigglypuff and
  `KirbyPurin` implement it. The two files compile to the same code (the
  same fused sites at the same offsets); what differs:
  - the attribute block is Kirby's +2B4..+358, Jigglypuff's +34..+D8 byte
    for byte but two words: the hitbox's minimum speed (+34C: 1.0 against
    her 2.0) and the turn's end ratio (+350: 0.7 against 0.25);
  - take_dmg_cb and death2_cb are Kirby's own (ftKb_PrSetCallbacks:
    ftKb_Init_800EE7B8 / 800EE74C, `damage_callbacks`), not the pose's
    restore, so a hit leaves the roll's scale and spin to the next motion;
  - the saved scale is u.kb.x8C, which ftKb_SpecialN_800F190C sets to
    (1, 1, 1) with every ability change;
  - the aerial roll's landing dust (efSync 0x3FF) passes `&mv.x34` (the
    direction) as its facing and `&mv.x14` (the roll angle) as its angle; the
    floor's atan2f is computed and dropped (retail 0x801054F8..80105514).
    With Jigglypuff's arguments `kirby_copy_rollout_air_fd_jigglypuff4`
    diverges in the particle dump on the landing tick.
  - deal_dmg_cb (fn_80100E0C) is `CharacterCallbacks::DEAL_DAMAGE` through
    `copied::deal_damage`; x21F8 (fn_80105978, the reversal after a cape
    turnaround) fails closed, as Jigglypuff's does.
  No article and no effect bank: 0x3FF and 0x406 are common effects.
  Unreachable, for both kinds (their attributes agree here): the aerial turn
  (AirTurn, SpecialN0) is entered only by a turn leaving the floor at more
  than +2F4 (6.5), and a turn starts at the roll's speed, which Release
  clamps to +2CC (6.0) and the landing gives at most 0.03 x (180 - 40) =
  4.2, then slows; and the aerial roll's floor bounce needs a vertical speed
  of +2FC (0.8) out of 0.4 times its fall speed, which the roll's terminal
  speed (+2C0, 1.3) caps at 0.52, so every landing rolls on.
  Evidence: eleven `kirby_copy_rollout_*_fd_jigglypuff4` scenes in
  `KIRBY_WITNESSES` (boundary `start_fd_kirby_jigglypuff4`, the swallow of
  `kirby_hat_jigglypuff_fd_jigglypuff4` first), timed on the port's dry runs,
  each recorded once and exact on its first gate with 0 differing
  particle-site ticks: rolls into Jigglypuff on the floor and from the air;
  a full charge, a turn, the roll off the edge and death in the aerial end;
  a roll running out on the floor; aerial charges landing rolling and landing
  charging; the aerial full charge; the stage's side bouncing an aerial roll
  back (death in the roll); the aerial start catching the ledge; Jigglypuff
  hitting Kirby in the charge, keeping and (by the draw) taking the ability.
  `corpus_v3_fd_kirby_jigglypuff4_e86fddb60_p2` (`CORPUS_V3_MATCHES`), which
  stopped at the aerial entry, is exact over its 3755 ticks.
- **The copied Chef** (`ft-kirby` `copied/chef.rs`): Mr. Game & Watch's
  states, moved into `ft-gamewatch-family` (`chef::ChefUser`), over Kirby's
  rows 510/511 with his food kind (It_Kind_Kirby_GameWatchChef, the same
  item logic) and attributes (+3E4/+3E8). Kirby needs a pan article
  (It_Kind_Kirby_GameWatchChefPan, `it-kirby` `chef_pan.rs`): the entry's
  accessory (fn_8010CFB0) hangs it on the left thumb with the damage
  callbacks and a hitlag pair, then hands accessory4 to the thrower. It goes
  when Kirby leaves the two rows (its own anim callback), on a hit or death
  (8010D0A8, the put-away arm), or with the ability. Its motion-change flags
  lack Mr. Game & Watch's SkipModel. Copy archive `PlKbCpGw.dat`,
  hat_dynamics[5] (food) and [6] (pan); no effect bank. The pan's hitlag
  pair is unreached (any hit on Kirby in Chef changes his motion first).
- **The copied Fireball, Megavitamin and green Fireball** (`ft-kirby`
  `copied/fireball.rs`; ftkirbyspecialmario.c, ftkirbyspecialluigi.c):
  Mario's states (`ft_mario_family::special_n`, now generic over a
  `Fireball` trait that every `MarioFamily` gets by a blanket impl) over
  rows 399/400 (Mario's hat) and 512/513 (any other: the entries and
  ftKb_SpecialNLg_800F951C / 800F9598 test hat.kind != MARIO); Luigi's
  (moved into the new `ft-luigi-family`, `fireball::GreenFireball`) over
  rows 431/432. Both throw from LHandN (Mario's and Luigi's own use
  L1stNb). fn_800F9260 returns before reading the throw flag without a
  copied kind; Mario's hat throws It_Kind_Kirby_MarioFire with efAlt 0x49F
  (EfKbMr.dat, bank 32, model 0x7D00), any other a Megavitamin
  (It_Kind_Kirby_DrMarioVitamin) whose colour is drawn over u.kb.x68 / x6C
  (800F190C resets both to 9), without a flash. fn_800F98F4 throws
  It_Kind_Kirby_LuigiFire with efAlt 0x4B1 (EfKbLg.dat, bank 37, model
  0x9088). The articles are the originals' item code under Kirby's kinds;
  the fireballs' bounces spawn efAlt 0x4A0 / 0x4B2 (generators 0x7D02 /
  0x908A) and Kirby's Mario fireball plays no bounce sound (the kind test
  at 8029B9BC). Copy archives `PlKbCpMr.dat` (ft_80459B88.x0->xC),
  `PlKbCpDr.dat` and `PlKbCpLg.dat`, hat_dynamics[0] each. No damage
  callbacks and no put-away arm. Evidence: boundaries
  `start_fd_kirby_{mario,drmario,luigi}4` and seven
  `kirby_copy_{fireball,pill,greenfireball}_*` scenes in `KIRBY_WITNESSES`,
  exact on their first gate with 0 differing particle-site ticks: two
  grounded throws into the copied fighter, an aerial throw ending in Fall,
  a short-hop one landing in the grounded row, a throw at the floor's edge
  sliding off into the aerial row (and death with the ability), a jab in
  the row whose draw keeps the ability and a taunt that drops it.
- **The charge flashes** (`charge_flash.rs`, ftKb_Init_UnkMotionStates4,
  800EE818): the colour Kirby's secondary colour slot takes whenever it
  empties, by the copied kind, while its kept charge is full: Donkey Kong
  58 (xBC == +190), Samus 54 (xA8 == +168, the integer converted), Mewtwo
  93 (x9C == +384), Sheik 87 (xB4 == 6). The port keeps it in
  `secondary_color_fallback`, refreshed by every charge-word write and by
  800F190C / the hat's change (every word 0: no colour). The arms of kinds
  not yet ported read their words at rest.
- **The copied Charge Shot** (`copied/samus.rs`): Samus's states moved
  into the family crate `ft-samus-family` (`charge_shot`, generic over
  `ChargeShotUser`; Samus and `KirbySamus` implement it), over Kirby's
  rows 407..412 (anim ids 346..351) with his ss attributes (+168..+178,
  Samus's +18..+28 word for word), the shot formed and fired at parts[44]
  (both; Samus uses RHandNb and ThrowN), colour 0x36 on reaching full,
  the muzzle flash efAlt 0x4A4 (EfKbSs.dat model 0x84D0, bank 34), and
  It_Kind_Kirby_SamusCharge (it-samus `SamusCharge<Kirby>`: the same item
  code; in hand efAlt 0x4A1, at full 0x4A2 / 0x4A3; owner rows 407..412
  without the cancel; copy archive PlKbCpSs.dat, hat_dynamics[0]). Two
  differences beyond the names: the charge's efLib_DestroyAll runs whether
  or not xAC is set (ftKb_SpecialNSs_DestroyChargeShot, so every frame of
  the cancel destroys Kirby's effects), and the put-away arm
  (ftKb_SpecialNSs_800FCD60, from 800F19AC and 800F1A8C) resets the kept
  level: the first hit after the swallow (death1_cb) takes a stored full
  charge. EfKbSs.dat's first texture group (CI8) carries a stale RAM
  address as its palette pointer; hsd-particle validates image pointers
  only. Witnesses: `kirby_copy_chargeshot_fd_samus4` and
  `kirby_copy_chargeshot_{lost,hit}_fd_samus4`
  (boundary `start_fd_kirby_samus4`).
- **The copied Giant Punch** (`copied/donkey.rs`): Donkey Kong's states
  moved into the family crate `ft-donkey-family` (`giant_punch`, generic
  over `GiantPunchUser`), over Kirby's rows 455..464 (anim ids 390..399)
  with his dk attributes (+190..+1A0, Donkey Kong's +2C..+3C word for
  word), the swings kept in u.kb.xBC, colour 58 at full, and the punch's
  models efAlt 0x4A9 / 0x4AA (EfKbDk.dat 0x9858 / 0x9859, bank 39, sync;
  the same scale, facing and animate-at-creation as efSync 0x4C8 / 0x4C9).
  Kirby's states arm ftKb_SpecialN_set_cbs and the efLib hitlag pair, but
  no take_dmg_2_cb (Donkey Kong's own destroys his effects on any hit in
  the move); the put-away (ftKb_SpecialNPr_80100DE0) drops the swings and
  Kirby's effects, so the first hit after the swallow takes a stored full
  charge. No article. Witnesses: `kirby_copy_giantpunch_fd_donkeykong4`
  and `kirby_copy_giantpunch_{lost,hit}_fd_donkeykong4` (boundary
  `start_fd_kirby_donkeykong4`, the hat's).
- **The copied Shadow Ball** (`copied/mewtwo.rs`): Mewtwo's states moved
  into the family crate `ft-mewtwo-family` (`shadow_ball`, generic over
  `ShadowBallUser`), over Kirby's rows 500..509 (anim ids 435..444) with
  his mt attributes (+384..+39C, Mewtwo's +0..+18 word for word), the
  ball at parts[39] (RHandN, retail 0x8010775C; Mewtwo's is his right
  shoulder), colour 0x5D at full, the full flash (efSync 27) four units
  above TopN (Mewtwo's seven), the charge in u.kb.x9C and the ball in x98.
  Unlike Mewtwo's file, every removal of the ball (cancel, roll, shield,
  put-away) and its release also run efLib_DestroyAll on Kirby
  (ftKb_SpecialNMt_80107040 and its inlined twins), the release's after
  the ball's launch. The put-away arms: 8010709C (death, taunt) drops the
  ball and the charge, clears the secondary colour slot (ftCo_800BFFAC);
  80107130 (a hit) keeps a full charge. The article is
  It_Kind_Kirby_MewtwoShadowBall (it-mewtwo `MewtwoShadowBall<Kirby>`: the
  same item code; owner rows 500..509 without the cancels; copy archive
  PlKbCpMt.dat, hat_dynamics[3], its pose and particle tracks as
  Mewtwo's). Witnesses: `kirby_copy_shadowball_fd_mewtwo4` and
  `kirby_copy_shadowball_{hit,lost}_fd_mewtwo4` (boundary
  `start_fd_kirby_mewtwo4`, the hat's).
- **The copied Thunder Jolt** (`ft-kirby` `copied/thunder_jolt.rs`,
  ftkirbyspecialpikachu.c): Pikachu's states in `ft-pikachu-family`
  (`special_n::ThunderJolt`, which every family member has by a blanket
  impl) over one marker, `KirbyJolt`, that picks the rows (429/430 for
  Pikachu's hat, 526/527 for Pichu's), the attributes (+1C0 / +1DC, the
  family's own block layout) and the sound by the hat's kind, as the
  callbacks' switches do. No scratch and no put-away arm. The ball and
  crawler are it-pikachu's under Kirby's kinds (`KirbyPikachu`,
  `KirbyPichu` owners; copy archives `PlKbCpPk.dat` / `PlKbCpPc.dat`,
  hat_dynamics[0] and [1], the crawler's pose sampled as for Pikachu's);
  the ball's trail is efAlt 0x4AD, particle kind 0x8CA0 of `EfKbPk.dat`
  (bank 36, which ftKb_Init_803CB46C names for both kinds). Swallowing
  Pikachu or Pichu needed nothing new. Witnesses `kirby_copy_thunderjolt_*`,
  `kirby_copy_pichujolt_*` (boundaries `start_fd_kirby_pikachu4`,
  `start_fd_kirby_pichu4`).
- Not ported: the copied neutral specials other than the Blaster, the
  Punch, the Shield Breaker, the Rollout, the Chef, the Fireball, the
  Megavitamin, the green Fireball, the Charge Shot, the Giant Punch, the
  Shadow Ball and the Thunder Jolt
  (`ftKb_Init_803C9DD0`; each kind's entry fails closed with its name),
  item pickup hand poses.

## Where this stands and how to resume (2026-10-12)

Exact against retail (125 scenes in `KIRBY_WITNESSES`, boundaries
`start_fd_kirby_fox4`, `start_ys_kirby_fox4`, `start_fd_kirby_captainfalcon4`
warm and cold): movement and the five midair jumps (bare and helmeted), the
common attacks with the dash attack's own rows and the rapid jab's fists, all
four throws (the three airborne ones with their mash-outs and Kirbycides),
hit reactions, death and revival; Inhale (whiff, catch, mouthful walk / jump /
turn, spit as a star, swallow, the copied kind as state), Final Cutter and its
beam, Hammer, Stone (with the damage shell and the slide on Yoshi's Story);
losing an ability by taunt, hit or death and its star.

Slippi (`public-v3.7-b`): four of the five Kirby games match every frame
(7001, 13271, 9109, 13809). `GAMEANDWATCH/18_09_37 Kirby + Mr. Game _ Watch
(FoD)` stops at tick 9909 of 15399 on Mr. Game & Watch's x in his aerial
Fire beside Kirby at the stage's left edge (not yet diagnosed); it stopped at
the copied Chef (5388) before it was ported and at the swallow (5100) before
the grafted hats.

Remaining work, in the order to take it:

1. **The five grafted hats**: done (2026-10-18). Boundaries
   `start_fd_kirby_{falco,jigglypuff,donkeykong,mewtwo,gameandwatch}4` and
   the five `kirby_hat_*` witnesses are exact. The fifth replay now runs to
   tick 5388, where Kirby uses the copied Chef (`ftKb_Init_803C9DD0`, item
   2). Open: the Dark color animation's flame (0x416 at bone 6) on a
   Kirby hit by Mewtwo's jab is placed about 2.4 behind his facing on the
   hit tick (particle dump only; RNG sites and the base gate are exact):
   `kirby_hat_mewtwo_fd_mewtwo4` tick 437 (port x 53.31, retail 50.93) and
   the hatless scratch scene `kirby_nohat_mewtwo_jab_fd_mewtwo4`
   (unregistered; port 48.61, retail 50.99). Not yet diagnosed.
2. **The copied specials**, one kind at a time (`ftKb_Init_803C9DD0` /
   `803C9E54`, rows 399..543, `ftkirbyspecial<kind>.c`). Done (wave 12):
   Fox's and Falco's Blaster and `ftKb_SpecialN_800F5C34`'s arm for a
   copied special's row (`kirby_copy_blaster_*`,
   `corpus_v3_fd_kirby_falco4_ed4320157_p0`); Captain Falcon's and
   Ganondorf's Punch (`kirby_copy_punch_*`, boundary
   `start_fd_kirby_ganondorf4`); Marth's and Roy's Shield Breaker
   (`kirby_copy_shieldbreaker_*`, `kirby_copy_flareblade_*`, boundaries
   `start_fd_kirby_marth4` and `start_fd_kirby_roy4`); Jigglypuff's Rollout
   (`kirby_copy_rollout_*`, `corpus_v3_fd_kirby_jigglypuff4_e86fddb60_p2`);
   Mr. Game & Watch's Chef (`kirby_copy_chef_*`: the held loop, aerial
   landing in it, a jab putting the pan away, a jab's draw taking the
   ability, a death in it).
   Done (wave 13): Mario's, Dr. Mario's and Luigi's Fireball / Megavitamin
   (`kirby_copy_{fireball,pill,greenfireball}_*`, boundaries
   `start_fd_kirby_{mario,drmario,luigi}4`);
   `ftKb_Init_UnkMotionStates4` (`charge_flash.rs`), Samus's Charge Shot
   (`kirby_copy_chargeshot_*`, boundary `start_fd_kirby_samus4`),
   Donkey Kong's Giant Punch (`kirby_copy_giantpunch_*`) and Mewtwo's
   Shadow Ball (`kirby_copy_shadowball_*`). The charge flash's Sheik arm
   (xB4 == 6, colour 87) waits for the copied Needle Storm.
   Next: Sheik, Peach, then the rest; each row's stale-move id exists
   already (`StaleMove::Copied*`). The pattern, per kind:
   - `crates/ft-kirby/src/copied/<kind>.rs`: marker types (`KirbyFox`,
     `KirbyFalco`) implementing the original family's trait over Kirby's
     state (rows, items, hand, attributes, scratch in `Kirby`, callbacks =
     `ftKb_SpecialN_set_cbs`); the states themselves stay in the family
     crate, generic over that trait (`ft_fox_family::special_n::Blaster`,
     which `FoxFamily` gets by a blanket impl). A kind whose original has no
     family crate makes its shared states generic first.
   - `copied/mod.rs`: the kind's `NEUTRAL` entry becomes `Ported`, its rows
     join `rows()`, and its arms of accessory4, the article owner view and
     the muzzle (each a switch on `hat.kind` in retail).
   - `ability.rs`: its arm of `ftKb_SpecialN_800F19AC` / `800F1A8C` (the
     put-away), and `reset_copied_scratch` (800F190C) for its words.
   - Articles: the item kinds in the original's item crate (as
     `it_foxlaser::KirbyFoxBlaster`), registered in `melee-lib`
     `scene_items` (`SceneItems` and `KIRBY_COPY_ARCHIVES`: the copy
     archive, its `hat_dynamics` slot per article, as
     `ftKb_SpecialN_800F16D0` registers them); effects from the copy's bank
     (`ftKb_Init_803CB46C[kind]`, e.g. EfKbFx.dat = bank 33) join
     `melee_ef::CHARACTER_EFFECT_FILES`.
   - A grafted hat's kind (Donkey Kong, Jigglypuff, Mewtwo, Falco, Mr. Game
     & Watch): `hat::grafted_parts` already makes the hat's parts present,
     which the copied rows' animations need.
3. **Inhaling items** (rows 357, 368, 370, 375, 379, 381; `fn_800F6178`,
   `ftKb_SpecialN_800F5DE8`) and **item pickup hand poses**
   (`ftKb_Init_OnItemPickup`).
4. **Branches that fail closed for want of a witness** (each `unimplemented!`
   names its C line): the aerial Final Cutter rows landing
   (ftkirbyspecialhi.c:447-453, 526-536); the beam on a slope, landing from
   the air, clanked, absorbed, shield-bounced (itkirbycutterbeam.c:93-97,
   128-133, 162, 167, 205-243) and beside a wall at rest (:138-143); an
   airborne throw landing before its flag (`fn_800DD5EC`, ftCo_Throw.c:207-223)
   and a throwing pair losing its floor (`fn_800DD684`, :225-234); the victim
   side's rarer partings (ftCo_CaptureCut.c:29-30, 115-122, 137-189;
   ftCo_Throw.c:534-538; ftCo_ThrownKirby.c:
   63-69, 76-83, 340-343); the mouthful sliding off a floor's end
   (ftkirbyspecialn.c:565-571, 591-598); Stone's steeper slides
   (ftkirbyspeciallw.c:177-188, 273-285, 308-313, 338-352) and a phantom hit
   on the shell (ftcoll.c:3666-3676); the ability star against a wall or
   ceiling (itgroundcoll.c:381-382). The explorer from the three Kirby
   boundaries (not yet run) and `melee-sim search` are the way to reach them.
   Done (wave 11): the held fighter's tap down through a platform
   (ftCo_CaptureWaitKirby.c:56-62, `kirby_capture_drop_ys_fox4`), and the
   holding accessory (fn_800DA190/DA4A0/DA678) keyed on the motion id, not
   the scratch: an inhale after a whiffed grab keeps Catch's scratch.
5. **No witness yet**: the wall-bounce star (`WALL_BOUNCE_EFFECT`), the
   bump-star cooldown in a state without a collision callback, the grounded
   Inhale rows leaving a floor, the spat star's hitbox on a third fighter.
