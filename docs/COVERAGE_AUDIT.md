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
| `ft-ness/yoyo` ftNs_AttackHi4_YoyoThink_IsRemove (ftnessattackhi4.c:460) | A yo-yo row past frame 2 with no article: `unimplemented!` (retail reads the user data of a NULL gobj) | Unreachable by the call graph (2026-10-24). The counter reaches 3 only through frame 2 of the same entry (the AttackHi4 / AttackLw4 Enters set 1; Charge, entered on frame 13, sets 0 and Release 14), where a failed Item_80268B18 sends Ness to Wait (ft_8008A2BC) before frame 3. After that fp->u.ns.yoyo_gobj is cleared only by the think's own despawn on the last frame (49 / 60; later frames skip the branch); the rows' Coll, after ft_80084104 has entered Fall; ftNs_Init_OnDamage from ftCommon_8007DB58, whose every caller (Fighter_ProcessHit_8006D1EC, ftCo_800C8D00, fn_800D290C, the capture and bind states) changes the motion; OnDeath; and the article's Anim (it_802BE958_inline), which ends it only with its owner outside motions 0x156..0x15B. The yo-yo's logic table has no hit, shield, reflect or collide callback, and as a held item (x13, it_802742F4) it skips the blast-zone and lifetime checks. A yo-yo left behind by re-entering a smash at IASA (frame 2 overwrites yoyo_gobj) lives on while Ness stays in the rows, so it cannot clear the new one there either |
| `melee-ft/fighter/commands.rs` rehit_hitbox | Ness's yo-yo rehit enabling hitbox 0 after the script's clear-all: ported (`ness_yoyo_stored_hitbox_fd_fox4`: a frame-12 hit, the clear at 32, the capsule back at 42 until the down smash's frame-44 clear; retail's +914 state and x2219_b3 agree). A capsule so revived that survives a motion change (x2219_b3 clear) is ported, unwitnessed | Scans of the up and down smash charged into Fox and his shield (also with a roll away), 1,200 candidates, found no revival outside the down smash's frames 42-43; Ness struck on those frames would carry it |

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
| `input.rs` special fall with jumps left | Likely unreachable: entry spends the jumps |
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
| `ft-donkey/special_n.rs` Giant Punch hitbox 1 without hitbox 0 | PlDk.dat SpecialN/SpecialAirN (submotions 322, 323, 327, 328): hitboxes 0 and 1 are always made together and only op 16 (clear all) removes them; no op 15. Now `unreachable!` |
| `ft-iceclimbers` ftNn_Init_8012300C / ftNn_Init_80123954, Nana's class | The port's `x4_flags` column covers every row a climber can be in (the common table and the 26 `SPECIAL_MOTION_FLAGS`); the Option is gone |

## Rolling specials after a cape, and Jigglypuff edges (2026-10-24)

| Site | Verdict |
| --- | --- |
| `ft-purin` x21F8 `ftPr_SpecialN_8014222C` (ftpurinspecialn.c:1447) | Ported (`ft_purin_family::rollout::cape_turn_end`) and witnessed exact: `puff_rollout_cape_turn_fd_mario4` (Mario capes a grounded Rollout; the turn ends in SpecialNRelease and the ball rolls back), boundary `start_fd_mario_jigglypuff4` |
| `ft-yoshi` x21F8 `fn_8013295C` (ftyoshispecials.c:1363) | Ported (`special_s::cape_turn_end`) and witnessed exact: `yoshi_eggroll_cape_turn_fd_mario4` (the turn ends in SpecialSLoop), boundary `start_fd_mario_yoshi4` |
| `ft-kirby` x21F8 `fn_80105978` (ftkirbyspecialpurin.c:1368) | Ported, unwitnessed: the same generic as Jigglypuff's (the asm of 80105978 and 8014222C differ only in the pi/2 literal's address: part 0, the same three fighter speeds and the same seven `mv.pr.specialn` words). A witness needs Kirby holding Jigglypuff's ability and a Mario or Dr. Mario cape (the only HitElement_Cape hitboxes), so three fighters; `make_boundary.py --players Mario Kirby Jigglypuff` saved `start_fd_mario_kirby_jigglypuff4` but its cold twin fails with "requires two fighters" (the scenario runner builds two) |
| `ft-purin-family` `ftPr_SpecialS_8013D8E4` enabling a capsule never seen | Unreachable as a retail state: it is the port's own condition (retail re-enables whatever x914[0] holds; the port remembers hitbox 0 from the roll's scripts). Both release scripts create hitbox 0 before the first `update_hitbox` of a roll, so the remembered capsule is always the roll's: with a probe, all 6,334 `update_hitbox` calls in the 201 `puff` scenarios and all 5,945 in the 168 `jigglypuff` ones (every Rollout and copied-Rollout witness and the corpus) found it remembered, the 6 re-enables in each included. The guard stays as an assertion |
| `ft-purin` `ftPr_SpecialHi_Anim` Pokémon Stadium sleep element | Out of the VS scope: `mv.pr.specialhi.x0` needs `gm_8016B1D8` (StartMeleeRules x5_3), which only event matches write (gmevent.c:403, from the event's x1_4 bit), and `grStadium_801D4FF8` (Pokémon Stadium's display in mode 8 on this player). Stays fail-closed for event mode |

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
||||||| parent of 46179860 (hsd-particle: close the fail-closed particle sites against a scan of every bank on the disc)

## Wave 13: particle banks on the disc (PARITY item 27)

Scan (2026-10-24): every `.dat`/`.usd` in `harness/roms/files` (1000
files), each `eff*DataTable` bank with particle commands (26: the
character banks, the Kirby copy banks, `EfCoData.dat`, `EfMnData.dat`) and
each stage `map_ptcl`/`map_texg` bank (24), 1037 descriptors in all, every
program decoded with `particle.c`'s operand lengths up to its `FF`. No
other public names a particle bank. Results and what they close:

| Feature | Disc data | Port |
|---|---|---|
| Opcodes | Only `00-3F 40-7F 80-9F A0 A2 A3 A5 A6 A8 A9 AB AC AD AF B1 B3 B6 B8 BA BB BC BD BE BF C0-CF D0-DF E0 E4 E5 E6 E8 E9 ED EF FA FB FC FD FF` occur | All ported; the other opcodes stay `UnsupportedOpcode` |
| Generator shapes | 0 (879), 2 tornado (14), 3 (12), 4 (1), 5 rectangle (49), 8 sphere (82); none of 1, 6, 7, 9-15 | Custom shapes (9-15) stay `UnsupportedGenerator`: retail calls `hsd_804D78EC`, which Melee never sets |
| Rectangle surface emission | No rectangle descriptor has a negative dimension; `rect.flag` is set only from the descriptor (generator.c:1155-1164) and no Melee code touches `gen->aux` | `Error::OutsideRetailData` |
| Camera-facing generator (kind 0x10000) | No descriptor sets it; only the `F0` opcode (unused) writes generator kind bits, and only 25-27 | An assertion, as retail: `psCamera` is only ever written NULL (generator.c:80), so `HSD_ASSERT(677, psCamera)` would halt |
| Descriptor AppSRT (kind 0x20000) | 84 descriptors (common, Fox, Ice Climbers, Kirby, Luigi, Mario, Pikachu, Pichu, Samus, Yoshi, Kirby's Pikachu/Pichu copies, stages `GrSt` and `GrZe`) | Allocated with `xA2 = 1` by `Generator::new` too; the camera-facing display cache (psdisp.c:1426-1438) is ported |
| Point joints (`B8`, `BF`) | Index 0 in all 13 uses (common 81/83, Fox 3000, Kirby 5004-5006, Mewtwo 13004-13006); `pJObjOfs` is only ever 0 (particle.c:474) | An index past slot 7 is `Error::OutsideRetailData` |
| Tornado particles | Kind bit 4 is in no descriptor: only a tornado generator's emission sets it, and `hsd_8039D3AC` (generator.c:92) keeps a generator while it has children | The "no live generator" errors became invariants (`expect`) |

`EfKbSs.dat`'s CI8 texture group 0 has neither `palflag` nor `palnum`, so
retail relocates a word of image data as its palette pointer; the bank
loader no longer rejects an out-of-range palette slot (it is never read).

## Wave 13: stage and camera sites (PARITY item 31)

Five fail-closed sites, audited 2026-10-24 against the decomp, the retail
asm and a scan of every stage archive on the disc (all 76 `Gr*.dat`/`Gr*.usd`
files with a `map_head`: each map's LightList through
`hsd_archive::visual::read_lights`, with `Ground_801C20E0`'s overrides
applied as `presentation/lighting.rs` applies them, and every
`HSD_AnimJoint` of every map animation walked raw for `HSD_A_J_PATH`
tracks).

- **Stage spline animation** (`melee-gr` `last/animation.rs`,
  `JObjEvent::Path`): proven unreachable, now `unreachable!`. The port
  raises the event only for a path track whose AObj has no object
  (`obj_id` 0); retail's `JObjUpdateFunc` asserts that object
  (`HSD_ASSERT(0x24D, jp)`, jobj.c:369-372), so such a track would halt the
  game. The scan finds 56 path tracks in stage map animations (GrBb,
  GrEF1-3, GrGd, GrKg, GrMc, GrNBr, GrNSr, GrRc, GrSt, GrTe, GrTFc, GrTGw,
  GrTMr, GrTMt); every one
  carries a relocated `obj_id`, which `attach_anim_joint` resolves to its
  spline joint or rejects at load. (Mute City map 30 animation 0 holds an
  unrelocated -1 child the strict reader refuses; the raw walk covers it.)
- **Battlefield demo/event background** (`melee-lib`
  `scene_stage/battle.rs`, map 5's proc): ported. `grBattle_OnInit`
  (0x80219CE8..0x80219D24) creates map 5 instead of maps 3 and 1 when
  `gm_8016B3D8` (Classic, Adventure, All-Star) holds or the stage kind is
  0x111 or 0xF5; its proc `grBattle_GObj5_Callback2` (0x8021A204) is a lone
  `blr`. VS and the replay runner never create map 5, and the Battlefield
  boundary loader still refuses any map order but `[0, 3, 1, 6]`, so the
  map 5 layout itself (its `Ground_JObjInline1` on-init, no transition
  controller) stays unported until a 1P mode runs.
- **Zoomed single-player framing** (`melee-cm` `standard.rs`,
  `Camera_8002B1F8`): still open, reachable only in modes the port does not
  run. The zoom `x2BC` is 1.0 from camera init (camera.c:193) and changes
  only in `Camera_8002B0E0`, under `gm_8016B41C` (Classic, Adventure,
  All-Star, Target Test, Training, Home-Run Contest, the multi-man modes,
  Event). VS and Slippi replays keep it at 1.0, where `Camera_8002B1F8`
  returns at once. The C-stick zoom (`Camera_8002B0E0`, already asserted
  off as `single_player_zoom`) and this branch need a 1P-mode boundary to
  witness.
- **Battlefield light kind outside ambient/directional/point**
  (`melee-gr` `battle/lights.rs`) and **animated light descriptors**
  (`melee-gr` `desc.rs`, `read_static_lights`): still open, reachable on
  stages whose simulation-side lights the port does not load yet. The
  static loader serves Battlefield map 6, Yoshi's Story map 3 and Dream
  Land map 5 (the `flags_b0` rows of `grNBa`, `grSt`, `grOp`
  StageCallbacks); their sets are ambient 0x4 plus infinite 0xD or point
  0xE, with no animation set and no active override. On the disc, after
  overrides, light flags are 0x4, 0x5, 0x6, 0xD, 0xE, 0x25, 0x40D and
  0x42D: the type bits are always ambient, infinite or point (no spot light
  anywhere), and the extra bits (hidden 0x20, 0x400, a dropped specular)
  come from `GrNBr.dat` maps 0, 1, 31-33 and Fountain of Dreams'
  overrides. Light sets with animations occur in 21 archives (GrBb, GrCn,
  GrCs, GrEF1, GrEF2, GrFs, GrGb, GrIz, GrKr, GrMc, GrNBr, GrNLa, GrOk,
  GrOp, GrPu, GrRc, GrSh, GrTPe, GrVe and the two `.usd` twins); among the
  supported stages they are the selected sets of Final Destination (map 3)
  and Fountain of Dreams (map 3), which only `presentation/lighting.rs`
  reads, and it ports the overrides and the LObj animation. A stage
  bring-up that loads its lights through
  `read_static_lights` must port these with it.

## Wave 13: effects (PARITY item 28)

Audited 2026-10-24 from main `1d722e33`. Two scans of the disc, read in
place: every effect table (`Ef*.dat`, 36 files) and every fighter script
(`ftData` `xC` motion table of all 33 `PlXx.dat`, decoded with
`ftAction_803C0870`'s lengths and followed through Command_05/07, plus the
color programs of `PlCo.dat`'s table at `ftLoadCommonData` +0x18).

**Closed as invariants** (`unreachable!`, not counted as fail-closed):

| site (line at `1d722e33`) | evidence |
|---|---|
| `melee-ef/src/resources.rs:341` particle commands without textures | All 36 effect tables link both particle commands and textures or neither (`efAsync_OnLoad`, efasync.c:1281). `crates/melee-ef/tests/disc_effect_tables.rs` checks it. |
| `melee-ef/src/lib.rs:2157` animation reference is not a spline | Of 138 effect models in 36 tables, one AObj names an object: effCommonDataTable row 36's PATH track, whose object is a spline joint. Same test. |
| `melee-ef/src/lib.rs:2412` unsupported ef animation event | The effect models' joint tracks are types 1-10, 12, PATH (4) and DPTCL (0x28): no SETBYTE/SETFLOAT (20-39) or PTCLTGT (0x2A) track, the only events without an arm. Same test. |
| `melee-ef/src/lib.rs:2018` AccumOffset on a freed item JObj | Retail call graph: efLib_CreateGenerator_AppSRT_SetPos parents the model to the item (`efLib_Create(0, gobj)`, eflib.c:1517) and Item_8026A8EC always runs `efLib_DestroyAll(gobj)` (item.c:1991) before the item is freed, so efLib_Cb_AccumOffset_FromParams never reads a freed JObj. The port's model has `ModelOwner::Item`, removed by `ItemEvent::DestroyEffects` and by the destroyed-item sweep in `frame.rs`. |
| `melee-ef/src/lib.rs:1561` unsupported fighter effect | A port dispatch default with no retail counterpart. Every `EffectRequest` constructor in the workspace was enumerated; the id-guarded rows cover each id passed: SyncAttached 0x47A 0x47C 0x482 0x488-0x48D 0x491-0x493 0x496 0x497 0x49B 0x49C 0x4B4 0x4B5 0x4B8 0x4B9 0x4BE 0x4BF 0x4C6-0x4CA 0x4D6 0x4DA 0x4DF 0x4EC 0x4EE 0x4EF 0x4F1-0x4F7 0x4FB-0x4FD 0x507 0x509 0x50A 0x50D-0x50F 0x510-0x512; SyncAttachedPair 0x48F 0x494 0x4AB 0x4B3 0x4BB 0x4BC 0x50B; AttachedParameter 0x490 0x495 0x49A 0x50C; FollowingGenerator 0x446 0x4E1; BoneModel 0x4CC. HitSpark reaches the switch only as Normal or Slash (`hit_log.rs` maps Ground/Cape to Normal and drops the no-effect elements). |

**Graphics ids a fighter can issue.** The scripts and color programs hand
ftCo_8009F834 these ids (opcode 10, the landing opcode 55, color opcode 21;
the terrain opcodes 54/55 and ftCo_DownBound add only stage-bank 30xxx ids
and 0x407): below 0x250, and 0x3E9, 0x3EF-0x407, 0x40C-0x413, 0x416,
0x41C, 0x41D, 0x422-0x424, 0x426, 0x428-0x42A, 0x438, 0x43A, 0x446,
0x44B, 0x487, 0x495, 0x49D, 0x4C1, 0x4C2, 0x4C4, 0x4C5, 0x4D1, 0x4D9,
0x4E1-0x4E6, 0x4FE, 0x500-0x502, 0x513-0x515. Every one has an arm in
`resolve_graphics_while` except two, so the dispatch site stays open for
them:

- `melee-ft/src/fighter/effects.rs:328`, 0x438 (block_66: efAsync kind 3,
  `efLib_Create_Attach(0x22)` with SetScale_FromParamX): issued only by the
  Screw Attack item's motions (ItemScrew, ItemScrewAir, ItemScrewDamage,
  in 27 playable fighters' data and the wireframes' and Giga Bowser's). Retail reaches it in any item match;
  the port has no Screw Attack item, so it waits for that item.
- 0x410 (efLib_CreateGenerator 0x22A): only Giga Bowser's SpecialLwLanding
  (PlGk.dat motion 315), a single-player fighter outside the port.

**Still open, with what reaches them.**

| site | why it stays |
|---|---|
| `melee-ft/src/fighter/procs.rs:467` terrain footstep before the proc's motion change | Needs a footstep (opcode 54) followed by a graphics command in the same script step, a terrain with a footstep effect (the port loads two: Fountain of Dreams' and Pokemon Stadium's water, material 10), and a motion change later in the same proc. The scripts that pair opcode 54 with a graphics command in one step are Bowser's walks (motions 7-9) and the heavy-item walks (motions 90/91 of 20 playable fighters' data). Walk's anim callback only sets the rate; the one proc-internal motion change found is ftCo_LiftWalk_Anim -> ftCo_80096F48 (ftCo_Lift.c:93-98, 78-91), which starts the next LiftWalk step at the end of each step: crate and barrel carrying, not ported. ftCo_800D71D8's item catch (an item caught while walking) also changes motion there but was not attempted. |
| `melee-ef/src/dust.rs:149` efAsync_Dispatch unsupported dust | The default of efAsync_Dispatch's positional rows. Every id the port's callers pass is in `DUST_SPAWNS` or below 0x250 or 30xxx: the graphics ids above that are not model rows, the `PositionalGenerator` constants of the character crates, and the item rows (`spawn_positional`). Retail reaches the other efasync.c rows (0x3ED, 0x40A, 0x41F, 0x425, 0x444, 0x445, ...) through callers the port has not ported (items, stages); each closes with its caller. |
| `melee-ef/src/egg_shell.rs:137` item root effect | efAsync kinds 1/4 on an item root: only the Egg Throw burst and the fireball's bounce are ported; other items' requests arrive with their items. |
| `melee-ef/src/item_generators.rs:41` and `:133` | efSync_Spawn rows on item JObjs and bones: the ported items' ids are all rows; other items' rows arrive with them. |
| `melee-ef/src/lib.rs:314` item-following generator | efLib_CreateGenerator_AppSRT_SetPos on an item: 0x448 (Link's stuck fire arrow, itlinkarrow.c:643) is ported. it_80278800 (iteffect.c:94-104) also sends 0x446 to an item when an item's animation list asks for it; no scan of item scripts was made, and the port's item-script graphics stop first at `frame.rs` "it_80278800: item effect". |

## Wave 13: subaction, capsule and part-tree sites (PARITY items 29-30)

The evidence for the fail-closed sites of `docs/PARITY.md` items 29 and 30
that no data or caller reaches. Scans read the disc's files in place
(`harness/roms/files`) with a scratch scanner over `hsd-archive` and the
port's decoders.

**Subaction opcodes (ftAction_80073240).** Every motion-table row of the 33
fighter data files (`Pl??.dat`: the 26 playable kinds, Nana, the Master and
Crazy Hands, Giga Bowser, the wireframes and Sandbag; 10014 rows with a
script, row counts equal to the descriptors' `animation_count`) and the 23
Kirby copy files, following Call (5) and Goto (7), decoded with retail's
lengths (`ftAction_803C0870` for 10..=58, lbcommand for 0..=9): no script
holds opcode 9 (lbBgFlash), 22, 32, 33, 44, 47, 48, 57 or anything above
58. The counts of the others run from 4 (opcodes 39, 53, 58) to 10334
(opcode 2). Fighter scripts come only from these tables
(`ftData_80085CD8`), so `melee_cmd::decode` now refuses those opcodes and a
file holding one would fail its load; `Command::Unported` is gone.

**Sound behaviors (ftAction_80071B50, ftAction_80071FC8).** Opcode 17
uses behaviors 0 (1337 commands), 1 (1031), 2 (341), 3 (83), 6 (57) and 11
(5); opcode 54 only 0 (300); opcode 38 behaviors 0 (5) and 2 (38). The port
now maps every behavior retail plays or stops (0..=6, 10..=15) and plays
nothing for 38's 7..=15 (retail 0x800721E8, `bgt`). Opcode 17's other
behaviors leave the stream after two words (ftaction.c:653), which the
three-word decoder cannot follow; none occurs, and the decoder refuses one.

**Colour animations.** All entries of PlCo.dat's colour table
(`ftLoadCommonData` +0x18, ids below 0x7B) decode with
`smash::read_overlay`; the table now fails its load on one that does not,
instead of keeping an undecodable program for run time. The scripts
request ids 1, 6, 13, 14, 39-41, 44-46, 48-52, 55, 56, 59, 63-76, 78-85,
88-91, 94-98, 100-102, 104, 108-116 and (smash charge) 119.

**Hit capsules.**
- `ftColl_80078C70` site (`melee_coll::detection::first_contact`): its one
  game caller is the grab test `ftColl_80078A2C`, whose catch box contacts
  only an enabled capsule (`lbColl_80007ECC`; `first_hurt_contact` already
  skips any other for a Catch hitbox), so an invincible capsule cannot
  come back. The function now drops the state.
- `ft_8007C224` (thrown-victim capsule): its `default: break` is ported;
  only `ft_8007C17C` (state 1) and the switch itself (2, 3) write the
  state.
- `lbColl_80008434` on a capsule never made: the only caller is Ness's
  yo-yo rehit (`ftNs_AttackHi4_YoyoCheckTimedRehit`, 0x80114F0C), whose
  timer starts in `deal_dmg_cb` (0x80115C74) when the yo-yo rows'
  capsule hits; those rows (0x127..0x12A of PlNs.dat) make capsule 0 only,
  and their entry clears the timer, so capsule 0 was made in this life.
  The site is now an invariant `expect`.

**Part trees (ftParts_800753D4, ftParts_80075304).** `Fighter_804D6540`
(PlCo.dat `ftLoadCommonData` +0x14) has entries for three kinds only:
Link (part 68 under 67, type 0, the joint's root), Young Link (72 under 71,
type 0, root) and Kirby (thirteen: types 0-3, source indices 7-28). The
port's `graft_conditional_joint` serves Link's and Young Link's OnLoad
(`ftLk_Init_OnLoad`, `ftCl_Init_OnLoad`), and in all ten of their costumes
(PlLk{Nr,Re,Bu,Bk,Wh}, PlCl{Nr,Re,Bu,Wh,Bk}) the parent part is
uninstanced and childless and the graft succeeds. Kirby's entries are
used only by `ftKb_SpecialN_800EF040`, which the port models in
`ft_kirby::hat` without moving a joint (no bone table names those parts).
The five sites are now assertions of the OnLoad layout.

Still open: a scaled fighter's hurt matrix (`lbColl_80008248` with
`ftCommon_8007F804`) needs a fighter scale other than 1 (the Super and
Poison Mushrooms, not ported); two efAsync flushes in one proc with a sound
draw between them (fighter.c:951) needs a second motion change in the proc
that entered a motion whose frame-0 commands draw a sound: 283 rows do
(opcode 18 or 38 before the first timer), mostly the damage motions
(0x6E..0x83) and 0xF7..0xFA; no witness is recorded.
