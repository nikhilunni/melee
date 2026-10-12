# Fighter words retail leaves as stale heap (2026-10-12)

`HSD_ObjAlloc` (0x8037ABC8) hands back the 0x23EC-byte Fighter block
uncleared, and `Fighter_Create` (0x80068E98) with what it calls
(`Fighter_UnkInitLoad_80068914`, `Fighter_UnkInitReset_80067C98` through
`Fighter_UnkProcessDeath_80068354`, the kind's `OnLoad` and `OnDeath`) writes
only part of it. A word nothing writes holds what the heap held; a word that
is then read before any state writes it makes a match depend on memory no
scene parameter names.

## What the port does

- A savestate import restores the character variables from the saved Fighter
  (`CharacterCallbacks::restore_saved`), stale ones included.
- A scene built from parameters (a cold twin) used the port's defaults. It
  now takes the words listed under "Read before written" from the first
  record of its boundary's retail recording
  (`CharacterCallbacks::restore_stale_heap`, called by
  `InitialState::carry_stale_heap` when
  `ScenarioSource::has_boundary_recording`: a cold scenario that names its
  recording as `expected`, whose `tick.raw` trace is then a required file).
  Only those words: nothing else of the recording enters the scene.
- `Match::new`, a Slippi replay and any cold scene without a recording keep
  the defaults (zero). See "What a match without a boundary should use".

## How the stale bytes were found

`harness/stale_heap.py` (needs the recordings and the decomp split):

1. `runs`: every `start_*` recording's first record is each Fighter as
   created. The first fighter's block address depends on the stage alone
   (six addresses over 174 boundaries: Battlefield 80C5C800, Dream Land
   80C62660, Pokemon Stadium 80C64DE0, Final Destination 80C6B960, Yoshi's
   Story 80C6CE60, Fountain of Dreams 80C7DC20), and `make_boundary.py`
   reaches every match the same way from boot, so the block held the same
   bytes each time. A byte creation never writes therefore has one value for
   every character at one address and (usually) another at the next. By that
   rule 5149 of the 9196 bytes are stale at a match's first frame.
2. `characters`: the same test for one kind's character variables
   (+222C..+233F), then the functions of that kind's units in the split
   retail asm that load from or store to each stale word.
3. Each word with a reader was then read in the asm and decomp for a store
   on every path from creation to the read.

Limits. A byte that happened to hold the same value at all six addresses is
not listed (it reads the same everywhere after boot). The displacement scan
does not see an access through a computed address (`addi rN, rFp, off`), so
arrays and embedded structs are judged by their guard field. Kinds with one
boundary as first fighter (Kirby, Bowser, Ness, Mewtwo, Young Link, Pichu,
Roy) are classified from that one address.

## What is stale

Reviewed in the asm and decomp (a store precedes every read, or nothing in
the game reads it):

| Bytes | Field | Basis |
|---|---|---|
| +0918..+0A4B and the three capsules after it | the four `HitCapsule`s past their first word | `state` (+0914 + 0x138 n) is not stale; the 308 bytes after it are |
| +1A4C..+1A53, +2224 bit 6 | `grab_timer`, x1A50, x1A51, x2224_b6 | `ftCommon_InitGrab` (ftcommon.c:668-680) writes all of them before `ftCommon_GrabMash` reads |
| +2004, +2014, +2018, +201C, +2104 | item and status timers | Fighter_8006A360 (fighter.c:1486-1590) tests each only under its flag (x221D_b6, x197C, x1980, x2221_b4), which is not stale |
| +2218 bit 0 | `allow_interrupt` | `ftAction` sets it; every file that tests it clears it at the state's entry (ftCo_AttackAir.c:128 and the other attacks' entries; Peach's float aerial enters through `ftCo_AttackAir_EnterFromMsid`); `ftCo_Landing` reads its own motion variable |
| +2218 bits 2 and 5 | x2218_b2, x2218_b5 | b2: ftCo_Attack1.c:125 and the script write it before ftCo_Attack100.c:126 reads; b5: `ftColl_CreateReflectHit` writes it with `reflecting` |
| +221B bit 2 | x221B_b2 | `ftColl_8007B1B8` clears it when the shield bubble is made |
| +2223 bits 2 and 3, +2228 bit 0 | x2223_b2, x2223_b3, x2228_b0 | read only by the draw code (ftmaterial.c, ftdrawcommon.c) |
| +2227 bit 1 | x2227_b1 | read only by the bonus counters (ft_80087858 from plbonuslib.c) |
| +2228 bit 7, +2229 bit 0 | the last stick-smash direction | b7 is read (ftCo_SpecialAir.c:46-48) only while x676_x is below PlCo +224, and x676_x starts at 0xFE (fighter.c:628) and is zeroed by the store that writes b7; b0 has no reader |
| +221F bit 7, +222A bits 2 to 7, +222B | x221F_b7, x222A_b2.. | no reader in the decomp |

Not reviewed store by store (the port models these as state that starts at
its default; every cold twin and Slippi bridge gates with that, which is the
evidence that nothing reached reads them first):

| Bytes | Field |
|---|---|
| +0044..+0073, +00A4..+00AF, +0104, +0338..+058F | below the scan's range (small displacements are not unique to the Fighter) |
| +11C4..+15xx (28 bytes in every 0x4C) | `FighterHurtCapsule` positions, behind a computed address |
| +1844..+189B | the `dmg` hit record (direction, angle, hurtbox, position, element, source, x187C..x1898) |
| +19C0..+1A3B | shield, reflect and absorb bubbles, `ReflectAttr` |
| +1A54, +1A68, +1A7C..+1A87 | x1A54, x1A68, x1A7C |
| +20B0..+20FF, +2118..+2133, +216C, +2174..+217F, +2188..+219B | x20B0.., Ness's yo-yo charge, `unk_grab_val`, a thrown fighter's saved translation, x2188, `grab_cb` |
| +2200..+220F | `cmd_vars[4]`: the port carries them across motions as retail does; a state that read one no earlier state or script had written would see zero in a cold scene |
| +2340..+23EB | motion variables (typed scratch in the port) |

## Read before written

| Kind | Word | Evidence | Effect |
|---|---|---|---|
| Luigi | +222C `x222C_cycloneCharge` | `ftLg_Init_OnDeath` stores only +2234 (0x8014230C); `ftLg_SpecialAirLw_Enter` loads +222C at 0x8014475C; only the Cyclone's landing and animation end write it | the first aerial Cyclone takes the charged branch (no `tap_y_vel_max` subtracted) when the word is not zero: `slp_bf_luigi_iceclimbers_t2300`, tick 540, 0.8 higher |
| Dr. Mario | +222C, +2230 `x222C_vitaminCurr`, `x2230_vitaminPrev` | `ftDr_Init_OnDeath` stores +2234..+2240 (0x801494C0..CC; `ftMr_Init_OnDeath` sets both colours to 9); `ftMr_SpecialN_VitaminRandom` (0x800E0D1C) loads both | the first two pills exclude a colour only if a word is 0..8. The draw count does not change (`HSD_Randi` once either way), so only the colours differ |
| Samus | +2234 `x2234` | `ftSs_Init_OnDeath` (0x8012832C) leaves it; `ftSamus_destroyAllEF` (in `ftSs_SpecialN_801291F0` and `ftSs_SpecialN_80129258`) tests it before a charge has set it | a nonzero word costs one `efLib_DestroyAll` the first time a shot is let go or dropped; no recorded scenario shows a difference (of the 69 Samus, Dr. Mario and Luigi cold twins, the 64 that gated exact still do with the words carried; the other five fail as before, for their own reasons) |
| Jigglypuff | +222C | `ftPr_SpecialAirS_Phys` increments it (ftpurinspecials.c:106); nothing else loads it | none: not carried |

Reviewed and written first: Yoshi +222C..+2237 (the Egg Roll's entry reads
the joint's scale into it, ftyoshispecials.c:317), Jigglypuff +2230..+223B
(the same in Rollout, ftpurinspecialn.c:238), Peach +2230 (the float's
timer, written when a float starts), Popo and Nana +2230 bit 0 (their
`OnDeath` clears it), Link and Young Link +2230 (`on21EC`, installed at the
boomerang's entry, runs before the throw reads it), Ness +2230..+223B (the
up smash's entry), Sheik +2238..+229C (`ftSk_SpecialS_80110F70` when the
chain starts), Donkey Kong +2230 (no reader). Kirby's +2234..+2320 are his
copy abilities' variables, written by `ftKb_SpecialN_800F190C` and the
ability's entry; not reviewed beyond that (his specials are in progress).

Not reviewed word by word: the hit, hurt and bubble structs behind computed
addresses (above, by guard field only) and the CPU block (+1A88..+1FDB),
which the port initialises from `ftCo_800A101C` as retail does.

## What the block holds: after boot and after a match

After boot (every `make_boundary.py` boundary), the first fighter's stale
words by stage:

| Stage | Address | +222C | +2230 | +2234 |
|---|---|---|---|---|
| Battlefield | 80C5C800 | 0932038D | 07E99800 | 0C094003 |
| Dream Land | 80C62660 | (no kind leaves it among the boundaries) | 80000000 | 3FD33322 |
| Pokemon Stadium | 80C64DE0 | 0100F707 | 0400F2FF | 1E1180AB |
| Final Destination | 80C6B960 | 00000500 | 000F0000 | 1E000005 |
| Yoshi's Story | 80C6CE60 | 80C6F0A0 | 00000006 | 00000000 |
| Fountain of Dreams | 80C7DC20 | 00000000 | 00000000 | 00000000 |

So a Luigi on port 1 straight after boot starts with the Cyclone charged on
every stage but Fountain of Dreams (Dream Land unknown). The second
fighter's address depends on what the first fighter loaded (83 addresses
over the 174 boundaries), and so does what it holds.

After an earlier match: `sudden_death_start_fd_marth` is the Sudden Death
that followed a timed match (Marth and Fox, the same players and stage as
`start_fd_marth4`). Both fighters sit at the addresses they had after boot
(80C6B960, 80CB7840). Of Marth's 2299 words 97 differ, of Fox's 149: the
words creation writes differently for Sudden Death (damage +1830, +2074) and
stale words the earlier match wrote, which now hold its last values (hurt
capsule positions +11C8.., +187C, +2188/+218C, the motion variables
+2360..+2368, +2227 bit 7). Every other stale word still holds what it held
after boot (`stale_heap.py compare start_fd_marth4 sudden_death_start_fd_marth`).
A stale word is therefore: after boot, a constant of the stage (first
fighter); after a match in the same slot, whatever that match last wrote
there, or the boot value if it wrote nothing.

## What a match without a boundary should use

There is no retail constant. The evidence for zero:

- the console of `slp_bf_luigi_iceclimbers_t2300`'s replay took the
  uncharged branch at its first aerial Cyclone (the replay shows the port's
  height, 1.146), where retail in Dolphin from a booted boundary took the
  charged one (1.946);
- all 28 Luigi games of `public-v3.7-b` the runner supports match to their
  last frame with the word zero, as do the 29 of 30 Dr. Mario and all 36
  Samus games of both corpora (a pill's colour is not in a replay; the one
  Dr. Mario stop is the Stadium jumbotron).

A Slippi console does not reach a match the way `make_boundary.py` does, and
a word like Luigi's is zero again once a Cyclone has landed in an earlier
game in that slot. So `Match::new` keeps zero: it is what the recorded
consoles show, and a scene that must be retail from a particular boundary
names that boundary's recording.

## The block's own address (2026-10-12)

One branch reads a Fighter's address as data. `ftCo_800AC5A0` (the CPU's
steer in hitlag, behaviour 18) with no knockback, which is a pummel, skips
the stores of both stick values (0x800AC6D0 to 0x800AC744) and passes two
registers it never set:

- y is r30, which `ftCo_800B2790` holds the Fighter pointer in (0x800B27A4):
  the low byte of the fighter's address;
- x is r5 as the mode's think left it. For the Ice Climbers' partner that is
  `ftCo_800B0CA8`'s scratch use, the last call of `ftCo_800B101C`'s seek
  (through `ftCo_800B0E98`): the player's fighter's motion id when it is on
  a ledge, captured, teetering, on the revival platform, on a Warp Star or
  under a parasol (0x800B0CA8), else the partner's `item_gobj` when null
  (0x800B0DAC), else the held Item's address (0x800B0DB8).

`corpus_v3_fd_iceclimbers_peach4_ecad2716f_p0`, ticks 950 and 951: Nana, at
0x80C9D2A0, steers y = -96 (0xA0) and x = 0. The other 20 no-knockback
ticks of behaviour 18 in the recordings fall where the steer is off
(`reaction_timer % 120 <= 120 - 12 * level`) and show a neutral stick, so
the x rule rests on the asm and that one zero.

The port keeps the address in `CpuState::fighter_address`: a savestate's
and a boundary recording's own (`carry_stale_heap`), else, for a partner,
the address after boot where a recording shows it
(`melee-lib` `initial_state/heap.rs`). Nana's Fighter in the first record
of every recording that holds one:

| Stage | Created before her (costume) | Nana |
|---|---|---|
| Battlefield | Popo (0) | 80C8E1C0 |
| Dream Land | Popo (2) | 80C93FE0 |
| Pokemon Stadium | Popo (0, 1) | 80C966E0 |
| Final Destination | Popo (0) | 80C9D2A0 |
| Yoshi's Story | Popo (0, 1) | 80C9E340 |
| Yoshi's Story | Popo (3) | 80C9E380 |
| Fountain of Dreams | Popo (1) | 80CAF520 |
| Yoshi's Story | Jigglypuff (0, 4), Popo (0, 1) | 80CCD940 |
| Battlefield | Luigi (2), Popo (0) | 80CC9E40 |
| Battlefield | Falco (3), Popo (1) | 80CCED60 |
| Yoshi's Story | Falco (0), Popo (0) | 80CDF300 |
| Final Destination | Captain Falcon (3), Popo (0) | 80CE1700 |
| Fountain of Dreams | Captain Falcon (1), Popo (0) | 80CF3840 |

With Popo first she is 0x31940 past him on five stages whatever the
opponent (Fox, Falco, Peach, Jigglypuff, Samus, Marth, Yoshi) and 0x314E0
or 0x31520 on Yoshi's Story; behind another fighter the distance follows
what that fighter loaded. It is not derivable without the heap's history,
so a setup outside the table has no address and the branch stays
`unimplemented!` there, as it does for a held item's address. A console
that did not boot straight into the match (every Slippi replay) need not
have these addresses: only multiples of 0x20 are certain.
