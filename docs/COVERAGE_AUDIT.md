# Fox–Marth on Final Destination: coverage audit (2026-09-26)

Scope as in `MATCHUP_COMPLETENESS.md`: two humans, Fox and Marth, Final
Destination, stock rules with an optional timer, random items off, character
articles and Sudden Death Bob-ombs in scope. This audit classifies every
explicit port boundary (`unimplemented!`, `todo!`, fail-closed `bail!`) in
the crates that run this matchup, and the silent gaps found by reading the
retail callers of paths the port skips. A boundary is **reachable** when
legal inputs in scope can reach it, **out of scope** when only another
character, stage, mode or CPU can, and **investigate** when neither is shown.

Evidence of exactness for what is implemented: 48 generated full matches
(corpus v2), the nine corpus v3 matches that faulted the port (240 explored)
and the directed scenarios in `m5_gate`, the timeout and Sudden
Death path (`tests/match_endings.rs`), cold starts, bones, particles and
allocation gates.

## Sudden Death items: highest priority

Sudden Death makes common items reachable. Ported and gated
(`sudden_death_pickup_bomb_fd_marth`): the pickup search
(ftpickupitem_800942A0) before jab, jab follow-ups, rapid jab, side and down
tilt; LightGet; the held Bob-omb, the hand pose and item idle (Wait1_1); the
lit fuse in hand and the blast releasing it from the hand (it_8027429C), which
hits its holder (xDCD b5). Fighter hitboxes land on item hurt capsules
(it_802703E8), with the item's hit spark, knockback and hitlag
(it_80270E30, OnTakeDamageThink), so a smash detonates a falling Bob-omb
(`sudden_death_smash_bomb_fd_marth`). A neutral A throws a held item
forward (LightThrowF, ftCo_80095EFC, Item_8026AD20; the Bob-omb's thrown
state 10) (`sudden_death_throw_bomb_fd_marth`). With an item in hand, A (or
LR+A) is taken by the grab check first and ftCo_80095A30 aims the throw:
back, up and down tilt throws and the smash throws (PlCo +400 rate), gated
by `sudden_death_throw{b,hi,lw,f4,hi4}_bomb_fd_marth`; walking while holding
(`sudden_death_walkthrow_bomb_fd_marth`); jumping while holding and
ftCo_80095328's air throws, which switch to the ground throw on landing
(`sudden_death_airthrow_bomb_fd_marth`). Z on the ground throws forward
(`sudden_death_zthrow_bomb_fd_marth`); Z in the air drops the item
(ftCo_80095744, Item_8026ABD8, it_3F14_Logic6_Dropped) and locks the aerial
catch until a grounded motion entry (`sudden_death_airdrop_bomb_fd_marth`).
A launch while holding draws Fighter_8006CDA4's drop chance: Randi(PlCo
+418) under the damage knocks the item loose through Item_8026ABD8
(`sudden_death_knockloose_bomb_fd_marth`); otherwise the fighter keeps it
through the damage states (`sudden_death_launchhold_bomb_fd_marth`), and a
death destroys it (Item_8026A8EC in ftCo_800D331C,
`sudden_death_kohold_bomb_fd_marth`). A dead fighter (x2219_b1) skips hit
detection. Shielding keeps the item and A in shield throws it (ftCo_8009515C,
`sudden_death_shield{hold,throw}_bomb_fd_marth`); dashing keeps it and A
mid-dash is a dash throw (LightThrowDash, ftCo_800D8A38) or, in the first
frames, a forward smash throw (`sudden_death_dash{hold,throw}_bomb_fd_marth`).
Every Wait entry plays the item idle while holding (ft_8008A348).
Turning, crouching, rolls, running, landing, the aerial jump, taunts and
the Fox/Falco/Marth specials keep it. LR + A in the air catches a light item
in reach without a motion change (ftCo_800D7100 -> fn_800D6F58,
`sudden_death_aircatch{dash,shield}_bomb_fd_marth`).

While a fighter holds an item only the audited states
(`item_pickup::HELD_ITEM_STATES`: standing, walking, dashing, running,
turning, crouching, shielding, rolls, jumps, falls, landings, air dodges,
taunts, specials of Fox, Falco and Marth, the ledge family, the damage
family, capture and thrown states, and the throws) may run; entering any
other state fails closed. Throws out of Turn (`sudden_death_turnthrow_bomb_fd_marth`),
dash throws at an edge and out of a run shield's countdown
(`sudden_death_runshieldthrow_bomb_fd_marth`) and C-stick smash throws are witnessed. The
remaining gaps:

| Retail path | Port | Reachability |
| --- | --- | --- |
| Down states while holding | Fail closed at the motion change | A tumble landing holding a Bob-omb; at Sudden Death's 300% every launch KOs first |
| Tilts with a held item | Fail closed | Unreachable for a throwable item: A with the item is ftCo_Catch_CheckInput's throw first, and the C-stick is a smash stick by default |
| Specials of other characters while holding | Fail closed per character (SPECIALS_KEEP_HELD_ITEM) | Out of scope: other characters |
| A walking Bob-omb leaving the ground (states 2/4 -> 1) | The walk and turn rows are ported (`sudden_death_walkbomb_fd_marth`); walking or turning off the ground fails closed | A walking Bob-omb at an edge; its lit walk lasts at most the blink countdown |
| ftCo_800D705C's catch window (x209C, ftCo_800D71D8) | Ported for FallSpecial (its only reachable opener: elsewhere A requests an aerial first); a held FallSpecial is witnessed (`corpus_sd_s1_eea202b0d_p2`), an open window is not | An empty-handed fighter in FallSpecial pressing A beside a live Bob-omb |
| HeavyGet | Fails closed | No heavy item appears in scope |
| Item hitboxes versus item hitboxes (it_8026FE68) and inert item hitboxes | Fail closed; item hitboxes on item hurtboxes are ported (`sudden_death_bombchain_fd_marth`) | Two live item hitboxes meeting |
| Unlit Bob-omb states (idle, pickup, walk, throw) | Fail closed | Unreachable in Sudden Death: the rain lights every Bob-omb at spawn |
| A fighter launched into a ceiling (FlyReflectCeil, ceiling tech) | Ported: ftCo_800C1718's bounce and FlyReflectCeil (`corpus_v3_s0_ef4efb740_p1`), ftCo_800C23A0's tech into PassiveCeil (`corpus_v3_s0_ef4efb740_p1_ceiltech`) | Reachable outside Sudden Death: a launch under FD's underside |

## How the Sudden Death gaps were searched

Hand-made witnesses start from `sudden_death_fd_marth4_timer60.sav`; inputs
are searched first with `melee-sim dry-run` (the port alone, from a
recording of the same savestate) and only the winners are recorded. The
corpus explorer's `sudden-death` mode (`cargo run -p melee-replay --release
--example explore -- harness/roms/files <out> <count> <skip> sudden-death`)
plays random but cautious inputs from the retail Sudden Death boundary:
4,300 cases found three faults (all fixed and gated as `corpus_sd_*`), and
its recordings bridge to Dolphin like any corpus case. Bridging a sample
found one silent divergence (a jab continued from Wait or Walk inside the
jab window, now gated) and taught the decoder to encode stale NaN words.

## Explicit boundaries

### Reachable

| Site | Boundary | Note |
| --- | --- | --- |
| `melee-ft/fighter/grab_damage.rs` ftCo_8008EC90 | A launch on a grab pair: ported when both are launched, whichever member comes first (`sudden_death_grabbomb_fd_marth`, `sudden_death_grabbedhold_bomb_fd_marth`), when only the captor is (ftCo_800DCFD4, `sudden_death_grabbombcaptor_fd_marth`), when only the victim is (ftCo_800DE2F0, the captor takes PlCo +380's hit, `sudden_death_releasecaptor_bomb_fd_marth`) and when the captor is launched as its pummel lands (ftCo_800DE854, `sudden_death_pummelcaptor_bomb_fd_marth`). A light third-party hit on the victim fails closed, and so does the captor that keeps its grab (x1828 = 3, ftCo_Damage.c:861-868) | Unreachable in a 1v1: the only third-party hitboxes are items; a Bob-omb hits for 25 (17 after a wall bounce), not under PlCo +3C0's 6, and a captor's laser flies away from the victim it holds. The x1828 = 3 arm is not the knockback armour of Yoshi's double jump: inlineB0 is ftCo_8008E984's test on the captor, `allow_sdi && x221A_b3 && kb_applied < x18A8 + PlCo +140`, a second hit weaker than the knockback hit whose hitlag the captor is still in. A knockback hit on a holder releases the pair in the holder's own ftCo_8008EC90, so by this reading (the decomp, not a search) a holder is in such hitlag only if its grab landed as it was struck, and the victim must then take a hit under 6 on a later frame of that hitlag while the captor takes a second knockback hit. No witness and no search (2026-10-12); the port stops at any captor in a knockback hit's hitlag, since it does not keep x18A8 |
| `melee-ft/fighter/damage.rs` ftColl_80076ED8 | A third fighter's hit on a captured fighter | Needs three fighters: out of scope in a 1v1 |
| `melee-cpu/attack.rs` ftCo_800B732C, `edge_guard.rs` ftCo_800B7180 | A CPU's target on a ledge at or above it: the ceiling test and the 75 degree bearing are ported from the asm, and so is behaviour 8, which the answer 1 starts. The rest of ftCo_800B732C (xF9_b3 set: guarding a target off the stage, ftcpuattack.c:1278) stays `unimplemented!` | The answer 0 is witnessed (`corpus_v3_ps_iceclimbers_peach4_ec3145eb3_p2`, tick 5198), and the answer 1 with behaviour 8 on Yoshi's Story (`corpus_v3_ys_iceclimbers_fox4_ecad2716f_p2`, an explorer case of 2026-10-12: Nana lands on Randall at y -33.25 under the ledge Fox hangs from, 15.35 below his TopN and 2.4 across, takes behaviour 8 at 2822, jumps at him with the stick at (19, 125), drifts toward his x while rising and returns to her behaviour at 2855). The ceiling exit, the hop toward the target's x (xF8 bits 3-4 = 2) and the jump withheld under the top blast line are not witnessed. Where it can happen: the test needs a grounded CPU no higher than the target's TopN (0x800B738C), and a fighter in CliffCatch or CliffWait hangs below the ledge (recorded TopN: Popo and Nana 12.29 under it, Fox 14.4, Peach 17.25). That is under every floor of Final Destination and Battlefield, so the answer there is always 0; of the stages without disc reads only Yoshi's Story has a floor below a ledge, Randall on the lower half of his path. On Pokemon Stadium the explorer's own game reaches it under a transformation's ledge (seed c3145eb3, profile 2, tick 5559), where retail fed the same pads is another game, because a transformation follows its disc read. The xF9_b3 arm cannot run for the partner: its think (ftCo_800B0760, `partner.rs`) clears the bit every time, and only ftCo_800AECF0 and ftCo_800AFC40, the thinks of CPU players, set it |
| `melee-ft/fighter/life.rs` ftCo_800DC920 | A thrown fighter that dies: ported for the second climber vanishing with its leader (`corpus_v3_bf_iceclimbers_fox4_ea1a1c9aa_p1`, `..._ec1d4b39b_p0`); a thrown fighter crossing a blast line stays `unimplemented!` (ftCo_CaptureCut.c:112) | No recorded scenario (2,713 swept) and no replay of either corpus carries a pinned victim past a blast line |
| `melee-ft/fighter/life.rs` ft_0D31.c:61-67 | A frozen fighter leaving the top (ftCo_800D47B8 / ftCo_800D41C4: DeadUpFallIce, DeadUpStarIce, x2227_b6, the block's model through ftCo_80090AC0 and XRotN's spin while x68 is set) stays `unimplemented!` | Reachable: the freeze sets x2222_b3, so a block above the top line is a KO whatever its knockback. No recorded scenario (2,893 swept) and no replay of either corpus reaches it, nor did 1,800 explorer cases from the Ice Climbers boundaries on Final Destination, Battlefield and Yoshi's Story (2026-10-12, 150 seeds each; none of them faults any more). No directed search was made: a witness needs the block carried over the line (Yoshi's Story's is 168), by a Judge 8 or a Blizzard high above the top platform or a launch of the block at high percent |
| `melee-cpu/tumble.rs` ftCo_800AC5A0 | The steer with no knockback (a pummel) is ported: y is the low byte of the Fighter's address and x what ftCo_800B0CA8 left in r5 (`docs/PORT_NOTES/STALE_HEAP.md`, `corpus_v3_fd_iceclimbers_peach4_ecad2716f_p0`). `unimplemented!` where the port has no value: a partner holding an item (r5 is the Item's address, 0x800B0DB8), a scene with no recorded Fighter address for Nana, and any think but the partner's seek | The x rule has one witness, a zero; the player's fighter's motion id as x (it on a ledge, teetering, captured or on the revival platform while the partner is pummelled on a steering tick, `reaction_timer % 120 > 120 - 12 * level`) is ported from the asm and unwitnessed: of 22 no-knockback ticks of behaviour 18 in the recordings, 20 are not steering ticks |
| `melee-cpu/select.rs` ftCo_800ADE48, `behave.rs` ftCo_800AD7FC | The test for behaviour 14 is ported (0x800AE6A8: kinds 0..6, 0x22, 0x2A, and the Item's kind read as a motion id by ftCo_800A5980); the behaviour itself stays `unimplemented!` | Needs the partner holding a container, Bob-omb, Poke Ball or a stage enemy: with items off only a Sudden Death Bob-omb. A turnip is not used (`corpus_v3_fd_iceclimbers_peach4_e59d49e21_p0`) |
| `melee-ft/fighter/item_pickup.rs` HELD_ITEM_STATES | DamageIce, DamageIceJump, FlyReflectWall and Catch with an item in hand fail closed. By the asm their callbacks reach fp->item_gobj (+1974) only through exits the list already holds (ftCo_800C5240 from ftCo_80091854, ftCo_Fall_Enter and ftCo_DamageFly_Anim; ft_8008A348 from Catch's ft_8008A2BC), and the freeze drops only a heavy item (ftCo_8009750C) | No witness, so none is in the list. Catch: the one case that reached it was a port fault (a teetering fighter throws the item, `corpus_v3_ps_iceclimbers_peach4_e75fb4a9a_p0`): retail's ftCo_Catch_CheckInput throws a held item before it can grab. DamageIce and FlyReflectWall: explorer cases of 2026-10-12 with a turnip or Bob-omb in play, none reaching either on a stage retail can follow: Ice Climbers vs Peach on Final Destination (1,495), Mario vs Peach on Final Destination (535), Link and Peach boundaries (81), Sudden Death (441). On Pokemon Stadium (874 cases) the explorer's own game freezes a Peach holding a turnip once (seed ae02b66b, profile 1, tick 4320), but retail fed those pads is another game by then (`docs/PORT_NOTES/UNREGISTERED_SCENARIOS.md`) |
| `melee-ft/fighter/grab_escape.rs` ftCo_8008EC90 | Captured damage outside low capture or throw | Unreachable in a 1v1: the branch needs a hit from the captor (only pummels, in CaptureWait/CaptureDamage, and throws, in the Thrown states, which are ported) or a light third-party hit (see the grab-pair row) |
| `melee-ft/fighter/damage.rs` fighter.c:2907 | Phantom contact and shield impact together: ported (the phantom branch precedes x19A4's, so the impact gets no response) | Unwitnessed: the phantom band is PlCo +7A8 = 0.01 of overlap; a directed Marth dtilt search against a shrinking Fox shield jumps from shield contact to 1.39 overlap |
| `melee-ft/fighter/damage.rs` ftColl_80076CBC | Simultaneous shield impacts: ported (the strongest impact wins; getEnvDmg rounding, a zero-damage hit sets none) | Unwitnessed: every Fox/Marth hitbox is in group 0 (a scan of PlFx/PlMs scripts), so only an item and a fighter hit together could do it |
| `melee-ft/fighter/clank.rs` ftColl_8007925C | An inert (element 11) hitbox touching an item | Unreachable: no element 11 in any Fox or Marth script, article or the Bob-omb (element 15 appears only in the out-of-scope item Swing states) |
| `melee-ft/fighter/down.rs` | DownReflect wall bounce; DownDamage wall tech/bounce | DownReflect out of DownBound is ported (ftCo_800C7CA0, fn_800C7DC4; three Pokemon Stadium replays reach it, docs/SLIPPI.md) and has no retail recording; out of a grounded DownDamage it is `unimplemented!`. Grounded DownReflect is unreachable on FD: the wall-hug flag comes only from the ECB side-point sweep, and a grounded ECB's side points sit above y = 0 while every FD wall is at or below it. An airborne DownDamage (a sub-7% hit on a prone fighter) reaching a wall needs a second hit sending it back under the stage: reachable in principle; ported from ftCo_DownDamage_Coll (a wall tech, else ftCo_800C17CC's wall then ceiling bounce), unwitnessed |
| `melee-ft/fighter/state/callbacks/collision.rs` ftCo_StopWall | Running into a wall | Unreachable on FD, for the same grounded-ECB reason as DownReflect |
| `melee-it/map.rs` it_80276D9C | An item pressed between two walls | Unreachable: FD's opposite walls are at least 107 units apart |
| `ft-fox-family/special_hi.rs` | Fire Fox platform skip | FD has no platforms: out of scope |
| `ft-koopa/special_s.rs` ftKp_SpecialS_801332C4 (ftkoopaspecials.c:214) | The Koopa Klaw's holder losing its floor (the pair drops): `unimplemented!` | Unreachable on FD: the hold, bite and throw rows keep their floor through ft_800827A0, which stops at an edge, and FD's floor does not move away |
| `ft-koopa/special_n.rs` ftKp_SpecialAirNEnd_Coll (80135714) | The aerial Fire Breath's end row landing in the grounded end: ported with the helper the start and loop rows use (`bowser_breath_air_start_land_fd_fox4`, `bowser_breath_air_fd_fox4`) | Unwitnessed: a `melee-sim search` over jump, double-jump and breath timing, 324 candidates, none reached (the end row outlasts Bowser's airtime by about 4 frames) |
| `ft-koopa/special_n.rs` ftKp_SpecialNLoop_Coll / ftKp_SpecialNEnd_Coll | The grounded Fire Breath loop and end rows sliding off an edge into the aerial rows: ported with the start row's helper (`bowser_breath_slide_off_fd_fox4`) | Unwitnessed: two searches over the run-up and breath timing at the edge, 32 candidates each, none reached |
| `ft-koopa/special_n.rs` ftKp_SpecialLw_80134ACC, `special_s.rs` ftColl_8007ABD0 | A scaled Bowser's flame offset and Klaw knockback: `unimplemented!` | Needs a model scale other than 1 (mushrooms, Giga Bowser): out of scope |
| `ft-koopa/special_hi.rs`, `melee-ft/fighter/capture_koopa.rs` | Whirling Fortress with a held item; a Klaw victim that holds an item or another fighter: fail closed | Items are off in the Bowser scenes; a victim holding a fighter needs three fighters |
| `ft-ness/special_lw.rs` ftNs_MS_SpecialLwTurn (371), ftNs_MS_SpecialAirLwTurn (376) | PSI Magnet's turnaround rows: `unimplemented_row` | Unreachable (2026-10-24): nothing enters them. In the whole DOL only the rows' own Coll changes to them (`li r4, 0x178` at 8011AA84 in ftNs_SpecialLwTurn_GroundToAir, `li r4, 0x173` at 8011AAE4 in ftNs_SpecialAirLwTurn_AirToGround); every other `li r4, 0x173` / `0x178` is Kirby's or Game & Watch's own motion or not a motion id, and no code outside ftnessspeciallw.s names a turn callback. Their counter mv+4 (turnFrames, fp+2344) is only decremented by the turn Anims (8011A75C, 8011A86C) and never set, unlike Fox's reflector turn (ftFx_SpecialLwTurn_Check, called at 800E8B28): a leftover of the shared reflector code with no entry |
| `ft-ness/yoyo` itNessyoyo_UnkMotion3_Phys (it_802BF800, itnessyoyo.c:255) | The yo-yo reeled in (motion 3): `unimplemented!` | Unreachable: the only call of it_802BFEC4 (motion 3) is ftNs_AttackHi4_YoyoThink_IsRemove's at 80115930, on the return frame (yo-yo attributes x44 / x4C); the same call ends the article (it_802BE958: Item_8026A8EC, HSD_GObjPLink_80390228) on frame 49 / 60, and PlNs.dat's yo-yo has x44 = 49 and x4C = 60 (re-read 2026-10-24: x40..x4C = 4, 49, 5, 60). Item_80268E5C with ITEM_ANIM_UPDATE runs no phys, so no phys callback of motion 3 ever runs |
| `ft-ness/yoyo` ftNs_AttackHi4_YoyoThink_IsRemove (ftnessattackhi4.c:460) | A yo-yo row past frame 2 with no article: `unimplemented!` (retail reads the user data of a NULL gobj) | Unreachable by the call graph (2026-10-24). The counter reaches 3 only through frame 2 of the same entry (the AttackHi4 / AttackLw4 Enters set 1; Charge, entered on frame 13, sets 0 and Release 14), where a failed Item_80268B18 sends Ness to Wait (ft_8008A2BC) before frame 3. After that fp->u.ns.yoyo_gobj is cleared only by the think's own despawn on the last frame (49 / 60; later frames skip the branch); the rows' Coll, after ft_80084104 has entered Fall; ftNs_Init_OnDamage from ftCommon_8007DB58, whose every caller (Fighter_ProcessHit_8006D1EC, ftCo_800C8D00, fn_800D290C, the capture and bind states) changes the motion; OnDeath; and the article's Anim (it_802BE958_inline), which ends it only with its owner outside motions 0x156..0x15B. The yo-yo's logic table has no hit, shield, reflect or collide callback, and as a held item (x13, it_802742F4) it skips the blast-zone and lifetime checks. A yo-yo left behind by re-entering a smash at IASA (frame 2 overwrites yoyo_gobj) lives on while Ness stays in the rows, so it cannot clear the new one there either. That overlap is ported: each instance keeps its own string, calls reach only `yoyo_gobj`, and any yo-yo's end clears the pointer with no identity check (YoyoItemSetFlag, ftnessattackhi4.c:563-569); witness `w25_ness_yoyo_overlap_v02` (binding replaced 877, bound yo-yo removed 924, older one ends itself 925) |
| `melee-ft/fighter/commands.rs` rehit_hitbox | Ness's yo-yo rehit enabling hitbox 0 after the script's clear-all: ported (`ness_yoyo_stored_hitbox_fd_fox4`: a frame-12 hit, the clear at 32, the capsule back at 42 until the down smash's frame-44 clear; retail's +914 state and x2219_b3 agree). A capsule so revived that survives a motion change (x2219_b3 clear) is ported, unwitnessed | Scans of the up and down smash charged into Fox and his shield (also with a roll away), 1,200 candidates, found no revival outside the down smash's frames 42-43; Ness struck on those frames would carry it |
| `melee-lib/scene_stage/kongo.rs` fn_801D8134, ftCo_8009EC44 | Kongo Jungle's barrel catching, holding and firing a fighter stays fail-closed, including a stationary barrel: the cited catch scenarios now stop under the moving-barrel guard and no completed witness covers use. The scene adopts main's shared barrel APIs; the N64 barrel witnesses remain exact. A fighter within twice the catch radius of a moving barrel also fails closed | Reachable and frequent: explorer round kj2 (2026-10-24, 60 seeds) shows the port's x730 (Ground_801C4D70 from grKongo_801D5774's lb_8000B1CC) a step off retail's while the barrel travels its path: a held Fox sits 1.48 behind (`corpus_v3_kj_fox_marth4_e0c8363c7_p1`, tick 1032), catches land a tick early or late (`_ef5e3e043_p0` 158, `_e255c070a_p1` 706, `_e8be4d273_p2` 1059). Next step: the barrel model's animation step against its proc (which runs first in retail) |
| `melee-lib/scene_stage/kongo.rs` grKongo_801D7BBC, Klaptrap | Kongo Jungle's Klaptrap event (map 11's path rider grKongo_801D651C/801D6668 on GrKg spline 4/5/6, itklap.c): `unimplemented!` | Reachable: 77 of 180 kj2 explorer cases stop here (`corpus_v3_kj_fox_marth4_e75fb4a9a_p0`, `_e89a89d0e_p0`, `_e0fcbde24_p1`, recorded). The unused, ungated WIP rider/tangent commit is excluded from this integration |
| `melee-lib/scene_stage/kongo.rs` grKongo_801D7BBC, box | Kongo Jungle's box event: `unimplemented!` | Creates a container item: with items off it is never drawn (its weight, yakumono +0xC, counts only when it_8026D324(It_Kind_Box) allows boxes, grkongo.c:1191) |
| `melee-lib/scene_stage/kongo.rs` grKongo_801D5340 | A second log (map 1) while the first lives: `unimplemented!` | Reachable once in 180 kj2 explorer cases (frame 3633); not bridged |
| `melee-gr/kongo` grKongo_801D577C | The barrel's keep case 1 with a NULL keep | Unreachable: the keep is set with the hold (fn_801D8134) and cleared only on launch |


| `melee-lib/scene_stage/kongo.rs` grKongo_801D6198, fn_801D7E60 | A log entering collision is witnessed by `corpus_v3_kj_fox_marth4_e0fcbde24_p0` and `_e11788094_p2`. A fighter standing on a log, sinking, and removal stay `unimplemented!` (grkongo.c:579,1282) | Guarding Land stopped both named full witnesses; enabling that transition alone restores complete exact gates. None of the eleven accepted Kongo/Japes witnesses reaches the retained standing/Sink/Remove guards |
| `hsd-anim/spline.rs` splGetSplinePoint / splArcLengthGetParameter | Linear and B-spline execution is ported; Bezier and Cardinal evaluation stays `unimplemented!` (spline.c:82-135,178) | Jungle Japes' main model carries a B-spline PATH joint, exercised by its warm/cold boundaries and full Klaptrap/river corpus witnesses. No Bezier/Cardinal retail witness is supplied |
| `it-kyasarin/lib.rs` it_8027C79C | A tumbling (knocked-out) Birdo meeting the map: `unimplemented!` (itzako.c:462) | Reachable in principle; 900 Mushroom Kingdom II explorer cases with branch tags (wave 27) knock Birdo out to the screen and tumbling but none tumbles into the map before a blast zone |
| `it-kyasarin/egg.rs` it_2725_Logic28_Reflected | A reflected Birdo egg: `unimplemented!` (itkyasarinegg.c:184) | Needs Fox's reflector against an egg in flight; not reached by the 900 tagged explorer cases |
| `melee-lib/scene_stage/greatbay.rs` grGreatBay_801F5AF8 | The moon's fall ending (AOBJ_REWINDED): `unimplemented!` (grgreatbay.c:834) | Unreachable by the disc's data: GrGb.dat map 9's loop flag is 0, so grAnime_801C8138 never sets AOBJ_LOOP; the 4690-frame fall stops on AOBJ_NO_ANIM. A 30000-tick idle port run starts the fall and never reaches the branch |
| `melee-lib/scene_stage/greatbay.rs` grGreatBay_801F63F4 | The turtle's item spawned with items on fails closed (grgreatbay.c:1349); the draw itself is ported | Needs an items-on Great Bay boundary; every registered Great Bay boundary has items off |
| `melee-lib/scene_stage/icemt.rs` (gricemt.c, Stage_80225194() == 76) | Adventure's Icicle Mountain: map 10's controller, the zako generator, the burst speed (x3A), the P1 height trigger. Not ported | Unreachable in Versus: grIceMt_801F686C takes the map 9 branch for every Versus StKind (only the Adventure stage id 76 makes map 10) |
| `melee-lib/config.rs` items_supported | Mushroom Kingdom II, Great Bay and Icicle Mountain refuse items (fail closed at setup) | No items-on witness yet; Great Bay's turtle item (grgreatbay.c:1349) is unported |

Kongo's Box event weight is currently excluded by items-off setup. Items-on
Kongo must remain rejected until the actual `it_8026D324(It_Kind_Box)`
eligibility is supplied to `tick_events` and the event's behavior is witnessed;
allowing other item stages must not silently retain `tick_events(false)`.

### Out of scope (other characters, stages, CPU or modes)

Character hooks without a Fox/Marth override (`fighter/mod.rs`: forward-smash
entry, throw callback, jab entry, tether, landing reset, morph-ball roll,
multi-jump table; `character.rs` taunt entry; `attack.rs` Link's second
forward smash; `jump.rs` non-basic double jumps; `damage.rs` double-jump
armor, DamageIce, model-scaled victims; `spawn.rs` scaled attributes;
`shield.rs` cape shield), CPU input (`input/human.rs`), platform drops
(`shield.rs` Pass, Reflector check-pass, grounded special fall), coin mode
(`fighter/mod.rs` attachment collision), other stages' backgrounds and lights
(`melee-gr`), the zoomed single-player camera (`melee-cm`), and item kinds
other than the Fox articles and the Bob-omb (`melee-it`).

### Investigations resolved (2026-09-27)

| Site | Verdict |
| --- | --- |
| `jump.rs` ftCo_Jump.c:69 | Unreachable: NTSC 1.02 stores only rumble per port (no tap-jump or jump-on-C option), and every `enter_knee_bend` caller first checks `human::jump_input`, which matches ftCo_Jump_GetInput (0x800CAE80) |
| `landing.rs` scratch inheritance | Ported for Walk, Dash, Run, RunBrake, Turn, Squat, KneeBend and WallJump (`reflector_runedge_jump_fd_fox`, `walljump_aerial_fd_fox`). TurnRun never writes mv+4 and keeps its predecessor's word, which the port does not retain: still fails closed |
| `procs.rs` ft_081B.c terrain footsteps | Unreachable on FD: the floor has flags 0 |
| `procs.rs` ft_0899.c pose paths | Unreachable on FD: the remaining path is body tilt, which needs a sloped floor or a floor line shorter than 5 (FD's are flat, 10.57/150/10.57) |
| `commands.rs` sound behaviors | Unreachable: a scan of every PlFx and PlMs script (following calls and gotos) finds only behaviors 0, 1, 2, 3 and 6, all ported |
| `color_overlay.rs` secondary programs | Unreachable: in scope only the smash charge (0x77) requests one with effects; 0x6C/0x6D come from item Swing states and 0x64 is Roy's |
| `special_lw.rs` Reflector turnFrames | Ported with the scratch words above; TurnRun as noted |
| `input.rs` special fall with jumps left (2026-10-24) | Unreachable; the IASA is now ported whole (ftCo_FallSpecial_IASA 80096AF4: parasol, catch window, ftCo_800CB870), so it fails closed nowhere. FallSpecial is entered only through ftCo_FallSpecial.c's inline0 (ftCo_80096900, ftCo_800969D8, ftCo_800968C8; Peach's SpecialHiStart writes `motion_id` but enters the parasol states). From the ground inline0 runs ftCommon_8007D60C (jumps = max); in the air ftCo_80096900 runs ftCommon_UseAllJumps, and ftCo_800969D8's only callers, Ness's PK Thunder 2 ends (ftnessspecialhi.c:962-970, 1002-1009, 1027-1034), run ftCommon_8007D60C first. The only other writers of x1968 are ftCommon_8007D5D4 (=1, leaving the ground), ftCommon_8007D6A4 (=0, landing), Fighter init and the Zelda/Sheik copy ftCommon_8007EFC8; none runs in the air during FallSpecial |
| `jump.rs` relaxed/C-stick entry, `input.rs` / `damage.rs` aerial IASA arms, `air_dodge.rs` tether kind (2026-10-24) | Unreachable by construction, now `unreachable!` or a plain return: `enter_knee_bend`'s callers (Wait IASA's `T::Jump`, the Reflector) test the same predicate as ftCo_Jump_GetInput; `fall::iasa_with_jump` yields only None, AirSpecial, Escape, Attack, Float and Jump, all handled by ftCo_Fall_IASA_Inner's and ftCo_Damage_IASA's ports; ftCo_800C3B10's kind test passes Link, Young Link and Samus, each of which installs `air_tether` |
| `grab.rs` fn_800DB6C8 hookshot (2026-10-24) | The site tested the captured fighter's own tether, but fn_800DB6C8's switch reads `victim_fp`, the captured fighter's `victim_gobj`: the captor (Link's it_802A7840, Samus's it_802BAA94). A captured fighter's tether is gone by then: ftCo_CapturePulled.c:45 runs ftCommon_8007DB58, whose death1_cb removes it (`ft_link_family::take_damage`). Now an invariant assert |
| `damage_bind.rs:54`, `damage_song.rs:25`, `capture_mewtwo.rs:41` ftCo_800DD168 in a grab pair (2026-10-24) | Open, out of reach of a two-fighter match: Sing (DamageSong), the bind (DamageBind) and Confusion (CaptureMewtwo) must strike a member of a grab pair, and with two fighters the other member is the singer or Mewtwo, who is neither holding (in its special) nor held (a captured fighter runs no special). The Ice Climbers do not grab each other, so Nana cannot be the other member either. A third fighter (a 3- or 4-player boundary) reaches all three |
| `capture_yoshi.rs:214`, `capture_kirby.rs:614` an invincible fighter released from a mouth (2026-10-24) | Open; not reached without items. The catch (ftColl_80078A2C, ftcoll.c:1586-1587) refuses x1988, x198C and x221D_b6, and a swallowed fighter runs no state that raises them before the release (ftColl_8007B868 reads the same three); the one source independent of the victim's state is a Starman's ftColl_8007B7FC, which needs items |
| `life.rs` gm_80167320 final stock | Unreachable: the scene freezes before the timer can expire |
| `state/special.rs` buffered special | Fixed: ftCo_SpecialAir_CheckInput (8009665C) reads the stick with inclusive bounds; the port's reuse of the ground's strict tilt timers took the side special at exactly (0.6, -0.55) (`airspecial_bound_fd_marth`) |
| `ft-purin/hat.rs` costume hats (2026-09-28) | The hat's pose and spring chains are not simulated: they reach no simulation state. The hat joints are no fighter part (ftCo_8009E318 matches part joints only); ftCo_8009E140 and ftCo_8009E7B4 select set 0 alone for Jigglypuff; the x594_b4 table path that indexes parts by a set's bone id needs ftData +2C +10, null in PlPr.dat, and no Jigglypuff motion sets 0x08000000; the hat root matrix is copied at display (ftPr_Init_UnkMtxFunc0). Witnessed exact with part bones in all four costumes (`puff_hat_c{1..4}_{rollout_ko,rest_hit}_fd_fox4`) |

The two remaining tests of the grab routine (`ftColl_80078A2C`, port
`grab::candidate`), read against the asm on 2026-10-03:

- `is_grabbable` (0x80078B7C..0x80078B84, per hurt capsule before
  `lbColl_80007ECC`): the port honours it, `melee_coll::detection::
  first_hurt_contact` skips a capsule that is not grabbable for a Catch
  hitbox (the flag comes from ftData and is clear on Samus's morph-ball
  capsule and on a fighter held in a mouth or an egg).
- `lbColl_8000ACFC` (0x80078B5C..0x80078B6C, per catch box before the hurt
  loop): a box skips a fighter its victim list holds. The list of a Catch
  hitbox is written in two ways only. The routine itself records the
  victim on a contact with no wall between (`ftColl_80076808` mode 0,
  0x80078BDC); the first such contact always becomes `victim_gobj` (x216C
  starts at F32_MAX), `Fighter_UnkProcessGrab_8006CA5C` runs `grab_cb` on
  the same tick, and the motion change drops the hitboxes, so that record
  has no reader. Otherwise a hit or clank of another hitbox of the same
  group would list the victim (catch boxes themselves neither hit nor
  clank: 0x80078DA4, 0x80078E58), and no supported character has such a
  script: a scan of every loaded script of the 20 characters finds Catch
  hitboxes only in Catch (242), CatchDash (243), Falcon's and Ganondorf's
  Dive (307) and Yoshi's Egg Lay (295), each with Catch hitboxes alone.
  The port now makes the same list test, which no scenario can take.

Catch boxes against items (`ftColl_8007BC90`) are not a gap: the item side
requires `xDD0 b4`, which only stage enemies set. A grabber dying with its
victim (ftCo_800D331C, ftCo_800DD100) is ported and gated by corpus v3.

## Character rows with no retail entry (2026-10-24)

A row is "entered" only by `Fighter_ChangeMotionState`,
`ftCommon_GroundToAirStateChange`/`AirToGroundStateChange`, `ftCo_Landing_Enter`,
`ftCo_Turn_Enter`, `ftWalkCommon_800DFCA4` or `ftCo_8008DCE0` with its action
id. For each row below the decomp names no such call, and the retail asm of
the character's units has no `li r4, <id>` (nor an `addi` that reaches the id
from a table base). Rows whose four state callbacks are NULL or a lone `blr`
are transcribed (`state::empty_row`); the others stay fail-closed.

| Row | Callbacks | Evidence |
| --- | --- | --- |
| Mario AppealSR/SL (341/342) | NULL, ftCo_SM_None | Entered only by ftDr_Init_80149910, ftCo_800DEA28's FTKIND_DRMARIO arm. Transcribed |
| Link AppealSR/SL (342/343) | NULL, ftCo_SM_None | Entered only by ftCl_Init_80149318, FTKIND_CLINK arm (Young Link's milk rows). Transcribed |
| Luigi SpecialS2 (345) | `blr` (80143B18/58/60, 80143BE0) | No `0x159` in ftLuigi; Green Missile's launch is 347 and its aerial twin 351 is entered at ftluigispecials.c:717. Transcribed |
| Pikachu/Pichu SpecialS1 (345) | `blr` (8012590C/4C/54, 801259D4) | No `0x159` in ftPikachu (Pichu shares the code); Skull Bash enters 343/344/346/347 on the ground and 348-352 in the air. Transcribed |
| Peach SpecialSJump (356) | NULL | No `0x164` in ftPeach; the Bomber's jump is the aerial 360. Transcribed |
| Ice Climbers SpecialHi_2/_5 (363/366) | NULL | No `0x16b`/`0x16e` load in ftPopo or ftNana (366 is only the top of `cmpwi` range tests). Transcribed |
| Ice Climbers SpecialHi_3 (364) | ftPp_SpecialHi_3_* | No `0x16c` in ftPopo or ftNana: fail-closed |
| Peach SpecialHiEnd/AirHiEnd (362/364) | ftPe_SpecialHiEnd_* | No `0x16a`/`0x16c` load in ftPeach (`0x16c` only in ftPe_SpecialHi_NotActive's range test, 8011D69C): fail-closed |
| Donkey Kong ThrowFWait2 (360) | ftCo_CargoWait2_* | The cargo rows are reached from `x2CC->x4_motion_state` (351) + 0..8 and the throws' +10..13; only the heavy-item landing adds 9 (ftDk_MS_349, 800E07CC), to `x2CC->motion_state` (341), giving HeavyWait2 (350): fail-closed |
| Donkey Kong SpecialLwEnd1 (386) | ftDk_SpecialLwEnd1_* | No `0x182` in ftDonkey; Hand Slap is grounded only (Start 383, Loop 384, End0 385): fail-closed |

## Special edges proven unreachable by the disc's data (2026-10-24)

| Site | Evidence |
| --- | --- |
| `ft-gamewatch/special_s.rs` ftGw_SpecialS_GetRandomInt, no face | The draw is `HSD_Randi(total)` and the last ceiling is `total`, so a face is found whenever `total > 0`. PlGw.dat's ext attributes x34..x54 (`ftDataGamewatch` +4, then +0x34) are nine 1s, so at least seven faces always weigh in. Now `unreachable!` |
| `ft-donkey/special_lw.rs` ftColl_8007B8A8 on a missing Hand Slap hitbox | PlDk.dat SpecialLwLoop (submotion 334): hitboxes 0..3 (op 11) are made, then throw_flags_b3 is set (op 20) that frame and the next, and op 16 clears every hitbox a frame after the second flag; the flag is consumed in the same tick's callbacks. Now `unreachable!` |
| `ft-donkey-family/giant_punch.rs` Giant Punch hitbox 0 present with required hitbox 1 missing | Original PlDk.dat submotions 322/323/327/328 and **PlKb.dat** (`ftDataKirby`, not the copy archive) 393/394/398/399 create/recreate 0+1 in the same immediate command batch and remove only with op16 (all); no op15. Fresh punch entry clears old capsules; ground/air SkipHit + UpdateCmd retains the whole pair. DK/Kirby cannot dispatch the typed Ness yo-yo revival writer; Ness's real unregistered per-index revival remains intact. The partial-punch callback runs after the complete batch and scales only with hitbox0 present. Now an explicit invariant panic; malformed/custom states still fail. See [proof and limits](PORT_NOTES/GIANT_PUNCH_PAIRED_HITBOX_INVARIANT.md). |
| `ft-iceclimbers` ftNn_Init_8012300C / ftNn_Init_80123954, Nana's class | The port's `x4_flags` column covers every row a climber can be in (the common table and the 26 `SPECIAL_MOTION_FLAGS`); the Option is gone |

## Rolling specials after a cape, and Jigglypuff edges (2026-10-24)

| Site | Verdict |
| --- | --- |
| `ft-purin` x21F8 `ftPr_SpecialN_8014222C` (ftpurinspecialn.c:1447) | Ported (`ft_purin_family::rollout::cape_turn_end`) and witnessed exact: `puff_rollout_cape_turn_fd_mario4` (Mario capes a grounded Rollout; the turn ends in SpecialNRelease and the ball rolls back), boundary `start_fd_mario_jigglypuff4` |
| `ft-yoshi` x21F8 `fn_8013295C` (ftyoshispecials.c:1363) | Ported (`special_s::cape_turn_end`) and witnessed exact: `yoshi_eggroll_cape_turn_fd_mario4` (the turn ends in SpecialSLoop), boundary `start_fd_mario_yoshi4` |
| `ft-kirby` x21F8 `fn_80105978` (ftkirbyspecialpurin.c:1368) | Ported, unwitnessed: the same generic as Jigglypuff's (the asm of 80105978 and 8014222C differ only in the pi/2 literal's address: part 0, the same three fighter speeds and the same seven `mv.pr.specialn` words). A witness needs Kirby holding Jigglypuff's ability and a Mario or Dr. Mario cape (the only HitElement_Cape hitboxes), so three fighters; `make_boundary.py --players Mario Kirby Jigglypuff` saved `start_fd_mario_kirby_jigglypuff4` but its cold twin fails with "requires two fighters" (the scenario runner builds two) |
| `ft-purin-family` `ftPr_SpecialS_8013D8E4` enabling a capsule never seen | Unreachable as a retail state: it is the port's own condition (retail re-enables whatever x914[0] holds; the port remembers hitbox 0 from the roll's scripts). Both release scripts create hitbox 0 before the first `update_hitbox` of a roll, so the remembered capsule is always the roll's: with a probe, all 6,334 `update_hitbox` calls in the 201 `puff` scenarios and all 5,945 in the 168 `jigglypuff` ones (every Rollout and copied-Rollout witness and the corpus) found it remembered, the 6 re-enables in each included. The guard stays as an assertion |
| `ft-purin` `ftPr_SpecialHi_Anim` Pokémon Stadium sleep element | Out of the VS scope: `mv.pr.specialhi.x0` needs `gm_8016B1D8` (StartMeleeRules x5_3), which only event matches write (gmevent.c:403, from the event's x1_4 bit), and `grStadium_801D4FF8` (Pokémon Stadium's display in mode 8 on this player). Stays fail-closed for event mode |

## Kirby's fail-closed branches (2026-10-24)

The explorer from `start_fd_kirby_fox4`, `start_ys_kirby_fox4` and
`start_fd_kirby_captainfalcon4` (6,264 cases: seeds 0..96 on all three,
96..696 on Yoshi's Story, 96..496 on the two Final Destination ones) and
directed `melee-sim search` runs (`goal.fault` aims a search at an
`unimplemented!` message). Closed with witnesses: listed in
`PORT_NOTES/KIRBY.md`, "Where this stands", item 4. What stays fail-closed:

| Location | Boundary | Evidence |
|---|---|---|
| `it-kirby/cutter_beam.rs` itKirbycutterbeam_UnkMotion0_Coll (itkirbycutterbeam.c:138-143) | A wall beside a beam with no horizontal speed | Unreachable: the physics, which runs before the collision, always sets `vel.x = facing * speed * cosf(angle)` with `speed >= 0.01`; the angle is a floor's (`atan2f(n.x, n.y)` with `n.y > 0`, inside +-pi/2) or a turned flight's (0 or pi plus `atan2f(vy, vx)` of a nonzero `vx`), and no float angle has a zero cosine |
| `it-kirby/cutter_beam.rs` Logic7_Clanked / Logic7_Absorbed (:162-170) | A clank with the beam; an absorber (Ness's PSI Magnet) | Out of scope for the Kirby boundaries: no Fox or Falcon hitbox clanks with an item (the explorer's 6,264 cases met none), and no absorber is on these boundaries |
| `ft-kirby/special_hi.rs` ftKb_SpecialAirHiEnd_Coll (ftkirbyspecialhi.c:526-536) | The aerial landing row landing | Its physics is ft_80085134 alone (TransN, no gravity), so once the landing has left its floor (392, Fox pushing it off: `kirby_cutter_push_fd_fox4`) it floats: only a floor rising into it could land it. Searches on Yoshi's Story with Fox running into a landing on a platform's end: 10,086 and 8,118 candidates, none reached 392 there |
| `melee-ft/fighter/throw_flight.rs` fn_800DD5EC (ftCo_Throw.c:207-223) | An airborne throw landing before its script's flag | Its pass refuses platforms (fn_800DD6E4), so only a main floor higher than the take-off can meet the flight early. Yoshi's Story's slopes rise 3.5 over 17 units: 7,812 candidates (Kirby's forward, back and up throws of Fox on the right slope, every grab tick from 195 to 215 and every Fox position from 42 to 50) found none |
| `melee-ft/fighter/capture_kirby.rs` ftCo_CaptureCut_Enter (ftCo_CaptureCut.c:29-30) | A fighter out of Kirby's mouth landed by its release sweep | 10,000 candidates on Yoshi's Story (Fox mashing out of the mouthful while Kirby walks onto or down the right slope, every walk start 281..320, five speeds, every stop 1..50 ticks later) found none |
| `melee-ft/fighter/life.rs` ftCo_800DC920 (ftCo_CaptureCut.c:137-189) | A fighter dying pinned in Kirby's mouth, moved by its release before its death entry | Reached and recorded (`kirby_inhale_randall_pit_ys_fox4`: both fall off Randall under Yoshi's Story, Kirby inhales Fox in the air and they cross the bottom blast zone, tick 1358). Not ported: passing the thrower to the death entry (the release, then the effect position, ft_0D31.c:408, 434) still puts Fox 1.53 right and 0.34 up of retail's release point, which is his XRotN; refreshing XRotN's constraint from Kirby's TransN2 first does not change it. Undiagnosed |
| `ft-kirby/special_lw.rs` ftKb_SpecialHi_800F3B28 (ftkirbyspeciallw.c:308-313, 338-352) | A floor past fifty degrees (+F8); the slide at its 2.0 top speed (+100) | Unreachable on the Kirby boundaries: Final Destination is level and Yoshi's Story's slopes are 11.8 degrees and 17 units long. Down one, the push is `(1 + (1 - traction)) * n.x * 0.1` (at most 0.041 a frame on that slope) and a rock landing on it starts at 2.0 times `n.x` (0.41), so it leaves the slope below 1.4 |
| `ft-kirby/special_lw.rs` ftKb_SpecialHi_800F3570 (:177-188), ftKb_SpecialHi_800F37EC (:273-285) | A dip (the floor ahead slopes back); a slope turned against the slide within four frames | Unreachable on the Kirby boundaries: Yoshi's Story's two slopes fall away from a level middle, so a sliding rock always crosses level floor (inside +F4) between slopes of opposite sign, which ends the history scan with 0.9 first |
| `melee-ft/fighter/capture_kirby.rs` ftCo_800BDA74 (ftCo_ThrownKirby.c:82-83), ftCo_800BE7C0 (:340-343), `ft-kirby/init.rs` ftCo_800BDA50 (:63-69) | The spat star under a ceiling; a hit on a fighter out of a star; a Kirby with a copied ability inhaled | The last needs two Kirbys (no Kirby ditto boundary); a hit in the four ticks out of a spat star needs a third fighter or an article in flight then; no stage on the boundaries has a ceiling the level spat star can reach (the explorer's cases met none) |
| `melee-ft/fighter/capture_kirby.rs` (ftCo_CaptureCut.c:115-122), (ftCo_Throw.c:534-538) | A pair parted while the Inhale still pulls; an invincible fighter leaving the mouth | Not reached by the explorer's 6,264 cases or the Randall searches (a pull lasts a few ticks) |
| `ft-kirby/mouthful.rs` fn_800F697C / fn_800F6AC8 (ftkirbyspecialn.c:565-571, 591-598) | The pull and the gulp losing their floor | 1,260 candidates with Kirby and Fox on Randall as it runs into Yoshi's Story's side wall (Kirby's turn 1286..1300, B 4..45 ticks later) found none: Randall leaves Kirby before Fox, so the inhale starts in the air |

## Article control messages, Peach's pulls and the item engine (2026-10-24)

`docs/PARITY.md` work items 16-18. Each fail-closed site was read against
its retail callers; the ported ones are gated, the rest are either out of a
match's reach (evidence below) or left to the random-item work.

Ported and witnessed:

| Site | Witness |
| --- | --- |
| Toad's owner hitlag (onEnterHitlag / onExitHitlag -> it_802BDFA0 / it_802BDFC0 -> it_8026B724 / it_8026B73C): the counter's hitlag freezes Toad | `peach2_toad_counter_laser`, `peach2_toad_counter_melee` (the retail trace's Toad holds xDC8 x7 for the counter hitlag) |
| The parasol's owner hitlag (ftPe_SpecialHi_8011D620 / 8011D650 -> it_802BDBF8 / it_802BDC18) | `peach_upb_parasol_hitlag_hit_fd_fox`, `peach_upb_parasol_hitlag_shield_fd_fox` |
| A walking Bob-omb whose timer runs out unlit lights itself (it_8027F8E0 with xDE0 == 0: blink, the common item lifetime it_804D6D28->x30, the fuse) | `peach_pull_bombhei_walk_ps_peach` (Peach's pull dropped onto a Stadium platform: walk, edge turns, light, burn, blast) |
| A stage enemy struck by a Dark hitbox shows hit_effect_ids[Dark] (it_80270E30, efSync 0x416) | `corpus_v3_ys_ganondorf_fox4_e1c91f9f7_p0` (new boundary `start_ys_ganondorf_fox4`; Ganondorf hits a Shy Guy) |
| Toad's spores bounce off a shield (itPeachToadSpore_Logic68_ShieldBounced -> itColl_BounceOffShield) | `peach_toad_spore_shield_bounce_fd_fox` (without the bounce the port leaves retail at tick 230) |

Unreachable, by the retail call graph (the arm is now `unreachable!` with
the reason):

| Site | Evidence |
| --- | --- |
| The `_` arms of the article control callbacks: arrow, bow, boomerang, hookshot, milk (`it-link`), Disable, Shadow Ball (`it-mewtwo`), charge shot, grapple beam (`it-samus`), Yoshi's egg, Pikachu's Thunder bolt | The only callers of these kinds' functions outside their own files are their owners: itlinkarrow (it_802A8398, it_802A83E0, itLinkArrow_802A850C, it_802A8A7C from ftlinkspecialn.c, ftkirbyspeciallink.c), itlinkbow (it_802AF1A4, it_802AF304), itlinkboomerang (it_8029FDBC, it_802A013C, it_802A0534, it_802A07B4 from ftlinkspecials.c), itlinkhookshot and itsamusgrapple (state changes and removal from ftCo_0D8E/0D95.c, ftCo_AirCatch.c, ftCo_CatchWait.c, ftCo_CaptureWait.c, ftCo_CatchPull.c), itclinkmilk (it_802C8B28 from ftclinkappeals.c), itmewtwodisable (spawn and Logic67_Destroy from ftmewtwospeciallw.c), itmewtwoshadowball (it_802C5000, it_802C519C, it_802C53F0, it_802C573C), itsamuschargeshot (it_802B55C8, it_802B56E4, it_802B5974), ityoshieggthrow (it_802B2890, it_802B28C8, it_802B2A10 from ftyoshispecialhi.c), itpikachuthunder (it_802B1DEC, it_802B1DF8, it_802B1FC8, it_802B1FE8 from ftpikachuspeciallw.c). Every one maps to a handled `ItemControl` (or a launch or spawn request); no other variant is sent to these kinds |
| The Thunder bolt's and the Thunder Jolt crawler's `_` link-message arms | A bolt hears only its spawner's chain setup (it_802B1DF8) and the next bolt's arrival (it_802B22B8); the crawler hears only the ball's contact (it_802B3F88); `Unlinked` is handled by the scene |
| itLinkArrow_802A850C with the arrow no longer its archer's | The archer's release is the only caller, with xE0 set; a nocked arrow has no hitbox, so nothing reflects or absorbs it to change its owner (retail falls off the function's end there, returning garbage) |
| `melee-it` `engine.rs` item command `_` | `desc::read_block` decodes only the item opcodes handled; Command_00..09 stay in the shared script machine |
| `ft-peach` `special_lw.rs` pull of another kind | PlPe.dat's attributes x10..x2C list three rare kinds only: Bob-omb, Mr. Saturn, Beam Sword |
| fn_8027FCA8 / fn_8028007C with an unlit Bob-omb | States 5 and 6 are entered only by it_8027F8E0, fn_8028007C and fn_8027FCA8, each of which lights the fuse (xDE0 = 1) first; only itBombhei_Logic6_Spawned clears it |

Left fail-closed, outside character articles and Peach's pulls:

| Site | Why |
| --- | --- |
| `it-bombhei` EnteredAir (it_3F14_Logic6_EnteredAir), walk and turn leaving the ground, states 0, 1 (its physics and collision), 3 and 12 | A pulled Bob-omb reaches them only when its floor moves or vanishes under it (it_8026D8A4 stops a walker at edges); a randomly spawned Bob-omb runs them from its spawn (it_8027DE18 -> state 1 -> fn_8027DAC8 -> state 0 -> it_8027E978 -> state 3) |
| `it-dosei` itDosei_Logic7_EnteredAir's unheld branch and states 7-10 | A pulled Mr. Saturn is picked up at its spawn (itDosei_Logic7_PickedUp sets xDC8 x19, which nothing clears), so its EnteredAir always takes state 6; states 7-10 belong to a stage- or randomly-spawned one |
| `it-drmariopill` motion 4-6 (itDrMarioPill_802C09C4's held poses) | Its only callers are ft_0BEC.c / ft_0BF0.c (ftCo_800BED88, ftCo_800BF108), reached through ftdemo.c's cutscene table, not a match |
| `melee-it` `desc.rs` item opcodes 20, 22, 24, 25 | A scan of the scripts of every kind the 99 boundary scenes register (all characters' articles, Peach's pulls, the Shy Guys) finds no such opcode; scripts decode at load, so a supported kind with one would fail every scene |
| `melee-lib` `frame.rs` it_80278800 item effect ids other than 0x405 and 0x513-0x515 | The same scan finds item-script effects 0x513 (Yoshi's egg) and 0x514 (Bob-omb) only |
| `melee-it` `hurt.rs` it_8027B798's grounded launch | Its callers are the walking stage enemies (itzako.c and kin); the supported one, the Shy Guy, is never grounded |
| `melee-it` `map.rs` it_80275E98 other hold kinds; `logic.rs` trait defaults (launch, dropped, thrown, control, link, logic) | Kinds the port does not register (random items, Pokemon) |
| `melee-lib` `frame.rs` it_802706D0 inert hitbox | `INTERACTION_MATRIX.md`: no supported kind has an inert hitbox |
| `melee-it` `hurt.rs` an item struck by a ReDead (Leadead) element | Unreachable in the supported scenes: a scan of every fighter and item script the 99 boundary scenes load finds the elements Normal, Fire, Electric, Slash, Coin, Ice, Nap, Catch, Ground, Cape, Inert, Dark, Disable and Lipstick, never Leadead (the ReDead enemy's grab, itleadead.c, not a versus item) |
| `melee-lib` `frame.rs` it_802B4224 with no room | Item_80268B18 failing needs the item heap full |
| `melee-it` `map.rs` it_80276D9C wall pinch | Investigate: an item touching both walls in one pass (a narrow gap); not met in the searches above |
| Yoshi's stars bouncing off a shield (it_802B312C -> itColl_BounceOffShield) | Reachable but not reached: grids of Yoshi's down special landing beside Fox's shield (Fox shielding from 104, 160, 200 or 240; 2,000 landings) met the shield only head-on, the least incidence 2.91 rad against the 2.36 limit (90 + it_804D6D28 +E0 degrees) |
## Character hook defaults and fighter proc gates (2026-10-24)

With all 26 kinds registered, every default that failed closed in
`melee-ft/src/fighter/{mod,character}.rs` was audited against retail's kind
switches. Each is now either the switch's default arm (retail's code for
every kind not named) or unreachable, with the evidence:

| Default | Retail arms | Verdict |
| --- | --- | --- |
| `enter_special` | ftData_Special*[kind]: one row per kind | Required trait item: every kind supplies its entry |
| `ENTER_TAUNT` (ftCo_800DEA28) | Young Link, Dr. Mario, Ganondorf; default ftCo_800DEBD0 | Required trait item: every kind names its arm |
| `forward_smash_variant` (decideFighter, ftCo_AttackS4.c:145-166) | Ness, Peach, G&W (own entries: `FORWARD_SMASH`), Pikachu/Pichu (override) | Default arm `Standard` |
| `throw_variant` (ftCo_Throw.c:145-157) | Fox, Falco, Samus, Kirby, Yoshi (all override) | Default arm: no callback |
| `jab_variant` (decideAttack11, getMotionFlags) | G&W (`ENTER_JAB`), Pikachu/Pichu (override) | Default arm `Standard` |
| `on_landing` (ftCo_Landing.c:51-83) | Mario, Dr. Mario, Peach, Roy, G&W, Popo, Nana, Kirby, Mewtwo (all override) | Default arm: nothing reset |
| `escape_variant` (ftCo_Escape.c:83-85) | Samus's morph-ball roll (override) | Default arm |
| `multi_jump_animation` | read only when `multi_jump_attributes` is `Some`: Kirby, Jigglypuff (both override) | `unreachable!` |
| `SPECIAL_GRAB` (fighter.c:2602-2603) | ftCommon_8007E2D0's callers: Falcon Dive (Falcon, Ganondorf), Inhale and Kirby's copied Egg Lay, Yoshi's Egg Lay, Koopa Klaw, Confusion, Master/Crazy Hand (unregistered) | `unreachable!`: only those crates set `special_grab`, and each supplies the hook |
| `SPECIAL_RELEASE` | Falcon Dive, Koopa Klaw, Confusion (`special_throw_release`) | `unreachable!` |
| `TRANSFORMATION_ARRIVAL` (ftCommon_8007EFC8) | called only by Zelda's and Sheik's Transform | `unreachable!` |
| `OWNER_BLAST` | only Samus's bomb (it_802B5478) returns an `OwnerBlast` | `unreachable!` |
| `ARTICLE_ACCESSORY` / `ARTICLE_PHYSICS` / `ARTICLE_REQUEST` | only `OWNER_ACCESSORY` items (Ice Climbers' string, Sheik's chain), `OWNER_PHYSICS` (Ness's yo-yo) and `owner_request` writers (Link's and Young Link's boomerang and hookshot) | `unreachable!`: each owner overrides |
| `SET_PARASOL_ANIMATION` (ftCommon_8007E83C) | Peach's parasol (override); any other kind only while holding the Parasol item (ftcommon.c:1069 asserts otherwise) | Still `unimplemented!`: needs random items |

`Interaction` (fighter/mod.rs) kept nine gates (held items, grabs, death,
status effects, accessories, the effect queue, stage collision lists, the
overlap nudge, coin-match attachment) that nothing ever set: those features
live in their own owners (item, grab and life modules, the scene's pairwise
procs). The gates and `Status::require_supported` are gone.

`Fighter_ChangeMotionState` flags (`spawn.rs`): a scan of every
Fighter_ChangeMotionState call in the decomp that evaluates its flag
argument finds Ft_MF_SkipThrowException (ThrownKirby), Ft_MF_KeepAccessory
(EntryEnd, RebirthWait, Kirby's copied Marth), Ft_MF_KeepColAnimPartHitStatus
(DownBound, DamageFall, FlyReflect, ftCo_800C7CA0) and Ft_MF_FreezeState only
under x2222_b6 (CaptureDamageKoopa, ThrownMewtwo, Shouldered).
Ft_MF_SkipThrowException and Ft_MF_KeepSwordTrail stay fail-closed (no
ported caller passes them, so nothing witnesses fighter.c:962 or 1152);
KeepAccessory keeps a model JObj nothing simulated reads,
and the part hurt status of x2221_b1 is set only under gm_8016B014 (no player
in slots 2-6), never in a Vs match. Ft_MF_FreezeState stays fail-closed:
fighter.c:785 clears x2222_b6 and no registered kind's OnLoad sets it. The
`Wait`-while-airborne guard had no retail counterpart (fighter.c has no such
test) and is gone.

### The inherited scratch word mv+4 (2026-10-24)

Retail's motion variables are one union a motion change does not clear; a
state that writes no mv+4 (fp+0x2344) runs on, and passes on, the word its
predecessor left. The port reads it through `Fighter::inherited_scratch_word`
and failed closed wherever that was `None`. `melee-sim scratch-diff --word`
now compares that word with each tick's fighter memory in a recording; run
over every recorded scenario (2,848 with tick traces) it found the `None`
sources and they are modelled:

- a savestate's fighter in Wait (the start of most directed scenes) had no
  typed scratch: it is imported as `MotionData::Inherited(fp+0x2344)`;
- CliffCatch writes only the ledge id (`CliffState::caught_word`);
- the capture states: CaptureKirby's pos_offset.y, the star's deceleration
  (ftCo_ThrownKirby.c:109), the Klaw's untouched +2344 (kept from the
  victim's state before the catch), the shouldered and egg mash bools
  (ftCo_Shouldered.c:51/97, ftCo_YoshiEgg.c:136), Donkey Kong's carry and
  cargo throw (no write: the word before the carry);
- Wait out of a character special holds the special's word, not the typed
  scratch of the common state before it;
- ftPk_SpecialLw_SetState_Unk0 (the Thunder lead ending) writing mv+4
  from outside stays fail-closed for the states it was before (Thunder
  Jolt, Skull Bash, and the property-0 common states without a typed
  arm): no recording reaches those writes (Dash's x4 is now the int retail
  stores, ready for it).

After this the tally over every recording has no unmodelled tick for any
kind outside the death and revival states (0-13, from which no state
inherits: Rebirth leads to RebirthWait, which writes its platform target)
and Zelda's and Sheik's Transform arrival (356/358, 361-364), whose word is
the arriving form's own from before it slept (a stale heap word in a cold
scene). The fail-closed guards of Raptor Boost, Reflector, Illusion, the
Link, Marth, Peach, Pikachu and Jigglypuff specials are therefore
`unreachable!`; `retained_drop_timer` (landing.rs) stays `unimplemented!`
for the Transform arrival. `m5_gate`'s
`inherited_scratch_word_matches_retail_on_every_tick` keeps 21 recordings
exact on every tick.

The tally still lists value differences (port `Some`, retail another
word) where a character's special row has no `RETAINED_SCRATCH_WORD` arm
and falls back to the common scratch (Fox's Fire Fox 353-356, whose angle
then reaches CliffCatch; Squat 39 out of some predecessors). No gated
trace reads them; they are follow-up work, not fail-closed sites.

## Random items (2026-10-24)

Unreachable on the witnessed Item Switch stages (Final Destination and
Battlefield), from stage data and call graph:
- The Egg (It_Kind_Egg) as a random item: Ground_801C28CC's weight is 0 on
  Final Destination and Battlefield (GrNLa / GrNBa grGroundParam +0x6A
  times the row's +0x1A), so neither pick table holds it.
- The Box's floating state 8 (it_80286088 / it_802870A4): only Kongo
  Jungle's grKongo creates it.
- The Barrel's state 4: no ittaru.c code calls Item_80268E5C with 4.

Items-on setups on other stages or with more than two players stay
fail-closed in both cold setup and saved-state import until witnessed.
The warm guard reads the saved rules, including when the scenario omits them.

See docs/PORT_NOTES/RANDOM_ITEMS.md for the open worklist.

## Exit status

Remaining reachable boundaries: the Sudden Death item gaps in the table
above. Ported from the retail branch order but unwitnessed: an airborne
DownDamage fighter reaching a wall, and the shield-impact combinations. The shield-impact combinations are ported from the retail branch
order but unwitnessed. `INTERACTION_MATRIX.md` lists the reachable
transitions no gated trace covers yet.

## Link's down aerial, Link's bow and Yoshi's Egg Lay on items (2026-10-24)

- **`ft-link-family/attack_air.rs` lwOnAnim (800EB528), re-enabling a
  capsule that was off before the bounce: unreachable, now an `expect`.**
  lwOnHit is the down aerial's deal_dmg_cb, which fighter.c:2928 runs only
  when `dmg.x1914` is set, and x1914 is written only where an enabled x914
  capsule deals damage (ftcoll.c:655 on a fighter, itcoll.c:459 on an item).
  Both down-aerial scripts switch capsules 0, 1 and 2 on together, re-damage
  them together and clear them together (PlLk.dat AttackAirLw: on at frame
  13, SetHitboxDamage on all three at 16, clear-all at 65; PlCl.dat the same
  frames with Young Link's damages), so at any hit all three are on and
  lwOnHit keeps all three. Witnesses: `links_dair_fd_fox4` (Link) and
  `links_yl_dair_fd_fox4` (Young Link, two bounces, the second from the
  re-enabled capsules).
- **`ft-link-family/special_n.rs` ftLk_SpecialNStart_Anim, the arrow not
  made: needs an exhausted object heap; stays `unimplemented!`.**
  it_802A83E0 returns NULL only if Item_8026862C does. The arrow is hold kind
  8 (Item_802674AC: Mario_Fire <= kind < Unk4), which Item_8026784C never
  refuses; the remaining failures are GObj_Create and
  `HSD_ObjAlloc(&item_alloc_data)`, and item_alloc_data has no number limit
  (item.c:113; only db/dballoc.c sets limits, on other pools), so they fail
  only when HSD_ObjAllocAddFree finds the object heap full. The port's nock
  queues the arrow and never takes the branch.
- **`ft-yoshi/special_n.rs` rows 348, 350, 353, 355 (SpecialN1_1,
  SpecialN2_1, SpecialAirN1_2, SpecialAirN2_1): reachable in retail, blocked
  in the port by unported items.** The tongue takes an item only through
  ftColl_8007BC90, which skips items without `xDD0_flag.b4`; only
  it_80275270 sets it, called by monster and enemy kinds alone
  (itoldkuri.c, itleadead.c, itoctarock.c, itoldottosea.c, itnokonoko.c,
  itpatapata.c, itlikelike.c, itwhitebea.c, itzako.c). In versus Goomba,
  ReDead, Octorok and Ottosea come out of an opened Crate, Barrel or Party
  Ball (it_8026F6BC, from itbox.c, ittaru.c, itkusudama.c): one chance in
  `itPublicData +124` = 128, weighted 2:3:1:1 by +128..+134 (ItCo.usd),
  under its own cap (Item_804A0C64 +2C < +30). The rest belong to 1P
  stages. A witness therefore needs items switched on, a
  container spawn and opening, and the monster kinds, none of which the port
  has (its items are fighter articles, Shy Guys and Sudden Death's
  Bob-ombs, none catchable). The earlier note above that only stage enemies
  set `xDD0 b4` misses the container monsters.
### Additional wave 14 stage explorer evidence

Four further existing retail traces now enter the ordinary m5 corpus:
Japes `e0fcf0c70_p0` (2488 ticks), `e573e2d95_p0` (4102),
`eb17a6598_p0` (2706), and Kongo `e726cfdde_p2` (1666), all with
62 field keys exact and complete RNG ledgers with zero particle-site
differences. The Kongo particle dump is exact. The three Japes particle
state passes timed out after 1976, 2484 and 2010 records respectively;
they remain NOTCMP for particle state, not evidence of a full state pass.

Japes `corpus_v3_jj_fox_marth4_ef89b3e70_p0` is retained as an ordinary
scenario with 2038 exact retail field ticks. Its ledger stops after 1489
records (`ledger.raw.jsonl.err`: no scheduler tick for 120 VI fields) and
there is no particle dump. It is not registered in the strict m5 corpus:
`combat_gate_ticks` requires exactly 2038 ledger records. Particle sites
agree only over the available 1489 records; the remaining 549 ticks need
valid existing evidence or a separately authorized recording before a full
particle claim or ordinary corpus registration can be made. A separate named
m5 test retains all 2038 field ticks and strictly compares the 1489-row RNG
prefix, asserting both lengths without changing the full-ledger helper. No
recording was changed.


### Original Swing42 command-owner invariant (wave24)

The missing static Swing42 row in `fighter/item_swing.rs` is an explicit
invariant panic, not newly implemented gameplay. All six item-class swing
entries clear the entire throw word before installing the actor's own script
(retail `0x800CD170..800CD17C`). The sole followup caller is the shared swing
IASA: b3 plus continuously held A selects input3. Across all33 original
ordinary ftData owners, all792 common swing script graphs (744 nonnull) have
only six b3 writers: Captain animations110/114/118/122/126/130. Captain owns
private rows341..346; all other original owners cannot request this arm.
The committed asset test reproduces this with the existing archive and command
loaders, including all25 copy-data roots as a separate ownership domain.
Nana/Popo fallback, prior borrowed commands and the demo no-Input/link3 fence
are documented in [the proof](PORT_NOTES/SWING42_COMMAND_OWNER_INVARIANT.md).
Custom or corrupted state still panics; unsupported item kinds remain guarded.
No gameplay, function-count or recording gain is claimed.


### Original common S4 followup ownership invariant (wave24)

`fighter/attack.rs` keeps the missing static Smash42 capability as an explicit
invariant panic matching retail800CED8C. No input ordering or gameplay is
changed. All165 common S4 submotion graphs60..64 across33 ordinary command
owners have exactly three positive var0 writers: Link, YoungLink and Ness.
Ness's positive belongs to its own private bat IASA; only Link/YoungLink's
common-S4 callbacks can request the existing private341 followup. The typed
asset test retains the separate25 copy roots and the earlier Swing42 proof.
The full1132C+3header producer, borrowed-script, mode/demo,18outgoing-hook and
365writer closure is recorded in
[the proof](PORT_NOTES/SMASH42_COMMAND_OWNER_INVARIANT.md). Malformed/custom
state still fails loudly. The existing common IASA prefix-order difference
is explicitly retained; no full IASA parity or new function/gameplay gain is
claimed.
