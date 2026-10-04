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
| `melee-ft/fighter/grab_damage.rs` ftCo_8008EC90 | A launch on a grab pair: ported when both are launched, whichever member comes first (`sudden_death_grabbomb_fd_marth`, `sudden_death_grabbedhold_bomb_fd_marth`), when only the captor is (ftCo_800DCFD4, `sudden_death_grabbombcaptor_fd_marth`), when only the victim is (ftCo_800DE2F0, the captor takes PlCo +380's hit, `sudden_death_releasecaptor_bomb_fd_marth`) and when the captor is launched as its pummel lands (ftCo_800DE854, `sudden_death_pummelcaptor_bomb_fd_marth`). A light third-party hit on the victim and armoured members fail closed | Unreachable in a 1v1: the only third-party hitboxes are items; a Bob-omb hits for 25 (17 after a wall bounce), not under PlCo +3C0's 6, and a captor's laser flies away from the victim it holds |
| `melee-ft/fighter/damage.rs` ftColl_80076ED8 | A third fighter's hit on a captured fighter | Needs three fighters: out of scope in a 1v1 |
| `melee-ft/fighter/grab_escape.rs` ftCo_8008EC90 | Captured damage outside low capture or throw | Unreachable in a 1v1: the branch needs a hit from the captor (only pummels, in CaptureWait/CaptureDamage, and throws, in the Thrown states, which are ported) or a light third-party hit (see the grab-pair row) |
| `melee-ft/fighter/damage.rs` fighter.c:2907 | Phantom contact and shield impact together: ported (the phantom branch precedes x19A4's, so the impact gets no response) | Unwitnessed: the phantom band is PlCo +7A8 = 0.01 of overlap; a directed Marth dtilt search against a shrinking Fox shield jumps from shield contact to 1.39 overlap |
| `melee-ft/fighter/damage.rs` ftColl_80076CBC | Simultaneous shield impacts: ported (the strongest impact wins; getEnvDmg rounding, a zero-damage hit sets none) | Unwitnessed: every Fox/Marth hitbox is in group 0 (a scan of PlFx/PlMs scripts), so only an item and a fighter hit together could do it |
| `melee-ft/fighter/clank.rs` ftColl_8007925C | An inert (element 11) hitbox touching an item | Unreachable: no element 11 in any Fox or Marth script, article or the Bob-omb (element 15 appears only in the out-of-scope item Swing states) |
| `melee-ft/fighter/down.rs` | DownReflect wall bounce; DownDamage wall tech/bounce | Grounded DownReflect is unreachable on FD: the wall-hug flag comes only from the ECB side-point sweep, and a grounded ECB's side points sit above y = 0 while every FD wall is at or below it. An airborne DownDamage (a sub-7% hit on a prone fighter) reaching a wall needs a second hit sending it back under the stage: reachable in principle; ported from ftCo_DownDamage_Coll (a wall tech, else ftCo_800C17CC's wall then ceiling bounce), unwitnessed |
| `melee-ft/fighter/state/callbacks/collision.rs` ftCo_StopWall | Running into a wall | Unreachable on FD, for the same grounded-ECB reason as DownReflect |
| `melee-it/map.rs` it_80276D9C | An item pressed between two walls | Unreachable: FD's opposite walls are at least 107 units apart |
| `ft-fox-family/special_hi.rs` | Fire Fox platform skip | FD has no platforms: out of scope |
| `ft-koopa/special_s.rs` ftKp_SpecialS_801332C4 (ftkoopaspecials.c:214) | The Koopa Klaw's holder losing its floor (the pair drops): `unimplemented!` | Unreachable on FD: the hold, bite and throw rows keep their floor through ft_800827A0, which stops at an edge, and FD's floor does not move away |
| `ft-koopa/special_n.rs` ftKp_SpecialAirNEnd_Coll (80135714) | The aerial Fire Breath's end row landing in the grounded end: ported with the helper the start and loop rows use (`bowser_breath_air_start_land_fd_fox4`, `bowser_breath_air_fd_fox4`) | Unwitnessed: a `melee-sim search` over jump, double-jump and breath timing, 324 candidates, none reached (the end row outlasts Bowser's airtime by about 4 frames) |
| `ft-koopa/special_n.rs` ftKp_SpecialNLoop_Coll / ftKp_SpecialNEnd_Coll | The grounded Fire Breath loop and end rows sliding off an edge into the aerial rows: ported with the start row's helper (`bowser_breath_slide_off_fd_fox4`) | Unwitnessed: two searches over the run-up and breath timing at the edge, 32 candidates each, none reached |
| `ft-koopa/special_n.rs` ftKp_SpecialLw_80134ACC, `special_s.rs` ftColl_8007ABD0 | A scaled Bowser's flame offset and Klaw knockback: `unimplemented!` | Needs a model scale other than 1 (mushrooms, Giga Bowser): out of scope |
| `ft-koopa/special_hi.rs`, `melee-ft/fighter/capture_koopa.rs` | Whirling Fortress with a held item; a Klaw victim that holds an item or another fighter: fail closed | Items are off in the Bowser scenes; a victim holding a fighter needs three fighters |

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

## Exit status

Remaining reachable boundaries: the Sudden Death item gaps in the table
above. Ported from the retail branch order but unwitnessed: an airborne
DownDamage fighter reaching a wall, and the shield-impact combinations. The shield-impact combinations are ported from the retail branch
order but unwitnessed. `INTERACTION_MATRIX.md` lists the reachable
transitions no gated trace covers yet.
