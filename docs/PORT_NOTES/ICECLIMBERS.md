# Ice Climbers bring-up

## Two fighters for one player

Player_80031AD0 creates Popo, then Nana (plAllocInfo.b0 → x221F_b4). The
fighter list, procs and trace keys run Popo, Nana, then the other players
(`p0`, `p1`, `p2`). The roster (`Setup::roster`) expands a character into
its fighters (`SceneFighter::partner_for`); the savestate import, cold
start, frame registrations, camera, interface and presentation all walk the
roster, and per-player views use the player's own fighter
(`player_fighter_index`).

Nana is a CPU in a human slot (Player_8003248C: PdPmdat z == 0). The
CpuGate proc (Fighter_8006ABA0) runs `melee_cpu::think` (ftCo_800B3900) for
her. Her animation table only authors rows 313..320; ftData_80085FD4 plays
Popo's figatree for the rest (`AnimationFallback`).

Shared rules for a player's two fighters:
- overlap pushes (ftCommon_8007E0E4/8007DD7C/8007DFD0): same-player skip,
  Nana never pushes, Nana's depth step (PlCo +45C/+460);
- hits and grabs (ftLib_80086FD4, ftcoll.c:1664-1676): never each other; a
  thrown fighter hits anyone but its grabber's player;
- a captured fighter takes PlCo +128 of a third party's damage
  (ftColl_80076ED8 inlineB3);
- stock standings count players only;
- items remember whether the owner is the player's second fighter
  (`owner_secondary`), so each climber's ice block is its own.

## Nana's CPU (melee-cpu)

Ported: upkeep (800B33B0), mode 6's partner decision (800B101C, 800B0760
seek, 800B0918 record, 800B0AF4 replay), behaviours 0/1 (idle, walk), 2
(attack: 800B658C, 800B8A9C, 800B4AB0 with its sqrtf-for-t² quirk), 9 (hold,
800B683C), 10 (arrived, 800ACD5C), 16 (mash, 800AC30C), 18 (steer,
800AC5A0), the script interpreter (800B3E04), PlCo pData[22], item views,
mpIsland.

`ftCo_800ADE48`'s tumble branch tests an uninitialised r31 (800AE270); from
its only reached caller, ftCo_800B0760, r31 holds fp+0x1A88, so the switch
always happens.

Recovery (behaviour 4): ftCo_800A8DE4 (also the Belay recovery a
partner in rows 361..366 falls into) picks an island end once with
ftCo_800A4038 / ftCo_800A3908 (the fall's height after t frames uses
sqrtf(t), like 800B4AB0); ftCo_800A9904 then jumps toward it, drifts in
over the stage, or up-specials (ftCo_800A96B8's default, diagonal; the
climbers' up special does nothing for Nana).

Unported (fail closed): ranged attacks (800B9CBC), edge guarding
(800B732C), off-island movement (800AB224 tail), the special-cased kinds'
recoveries in ftCo_800A96B8 (Pikachu, Fox, Yoshi, Ness, Luigi, Zelda,
Samus), KO totals (gm_8016C75C), stage routes.

## Specials

Ice Shot (ftPp_SpecialN, it_802C1590/it_802C16F8, `it-climbersice`) is
exact on both climbers, grounded and aerial, including its generators
(bank 14, efsync 0x4E9..0x4EB on the block's child joint). efAsync_Spawn
from an owner's accessory4 (s_link 9) dispatches at once (efasync.c:1458).
Squall Hammer (ftPp_SpecialS*, ftnanaspecials.c; `special_s`) is exact on
the ground and in the air, alone (343/345) and linked (344/346 with Nana in
SpecialS_0/_1, 359/360): B-press lifts and landings, stick steering, running
off the edge, the wall rebound, Nana joining from the other ground state,
hits that share hitlag, and a trade that unlinks the pair.

- The callbacks reach into the other climber (Player_GetEntityAtIndex). The
  scene hands a climber its partner around each proc
  (`CharacterCallbacks::OBSERVE_PARTNER` before, `ACT_ON_PARTNER` after;
  `melee-lib` `frame/partner_fighters.rs`): Popo's entry decides from the
  observed Nana (ftNn_Init_80123954) and moves her after his proc; Nana's
  collision that stands on Popo's position copies his collision data after
  hers. Either climber's unlinking (Fighter_UnkSetFlag_8006CFBC, x1A5C =
  NULL) reaches the other the same way.
- x1A5C outside a grab is `HitlagLink::partner`: a hit that starts one
  climber's hitlag holds the other (Fighter_UnkRecursiveFunc_8006D044).
- The stale-move table is the player's (Player_GetStaleMoveTableIndexPtr):
  each fighter keeps a copy and the scene hands the newer one over before
  every callback; Nana numbers her attack instances apart from Popo's, as
  retail's single counter does. Nana's hit after Popo's is staled by his.
- mv.pp.specials.x8 is never set, so ftPp_SpecialS_8011F720 never acts; the
  aerial ceiling test (`(env & Collide_CeilingMask) == 1`) never holds.

Belay (ftPp_SpecialHi, `special_hi`): Popo's start (347/352) pulls Nana
in at the script's cmd_vars[2] when she is within x7C and free
(ftNn_Init_8012300C: not out of play, not in hitlag, her row's x2071 class
not 1, 3..8 or 10..13); otherwise the solo rows (350/355, 351/356). Nana
hangs from Popo's right hand (361, ftNn_Init_801230D0), is flung (365) and
lands (362); the throw's cmd_vars[1] sends Popo climbing after her (354,
ftPp_SpecialS_80120E68). The Belay reads and moves the other climber
through the Squall Hammer's OBSERVE_PARTNER / ACT_ON_PARTNER: the observer
also takes the proc about to run and the partner mutably, so that a hand's
world position (lb_8000B1CC on the partner's joint) is read only for the
proc that reads it (`special_hi::partner::wants`), and Popo's CollData only
for Nana's launch; PartnerWork carries the join and Nana's hand for Popo's
u.pp.x2240 (fn_80123218). The FtPart constants index `fp->parts` directly
(retail +0x1D0, +0x2F0, +0x20).

The rope (It_Kind_IceClimber_GumStrings, `it_climbersice::string`) is a
handle in Popo's left hand; its 40 links (`special_hi::rope`, boxed in the
payload) and their steps are Popo's ARTICLE_ACCESSORY. Only its reel-in
decides anything traced (state 3 back to 0). Removed by the Belay's frame
0x53 it plays no destroy effect; destroyed because Popo left the Belay it
plays the article's 0x421 (its cleanup clears the owner first,
item.c:1993). The Belay script's efAsync 0x44B is generator 0x237 at scale
1 (efasync.c:842-864).

Witnesses: iceclimbers_belay_fd_fox4 (grounded then aerial, joined),
iceclimbers_belay_solo_fd_fox4 (grounded, Nana still shielding),
iceclimbers_belay_offstage_fd_fox4, iceclimbers_belay_ledge_fd_fox4 (the
start catches the ledge; Nana lets go). The aerial solo rows (355/356) and
Popo's climb hitting a wall or ceiling have no witness: Nana copies Popo
six frames late, so an aerial Belay she cannot join needs her hit or far
away (a `melee-sim search` over Fox's approach and Popo's retreat found no
candidate).

The Blizzard remains unported.
