# CPU players in a Versus match (2026-10-24)

Wave 13, work items 1, 3 and 4 of `docs/PARITY.md`. Before this, `melee-cpu`
served only Nana (CPU mode 6); a CPU player failed closed at its first think.
Mode 4 is witnessed on Final Destination and Battlefield, with CPU players
across the registered roster and levels. Stage-specific routes, hazards and
the other unwitnessed branches below remain fail-closed.

## Static character choices (wave 14)

`CharacterCallbacks::CPU_PROFILE` supplies immutable data through each live
fighter's `CharacterTable`: ordered exclusion probes and script lists,
attack motion thresholds, recovery/ranged/follow-up strategies, charge loop
rows, defensive threat ranges, holding/finisher choices and scaled-jump
capabilities. `melee-ft::fighter::cpu_profile` owns only these small `Copy`
types; the shared CPU subsystem executes the semantic strategies without
classifying a fighter's kind. Character crates supply their own profile,
so the dependency remains character -> melee-ft and melee-cpu -> melee-ft.
Nana's partner capability excludes her from the leader's scripted finisher.

The existing neutral-charge state hook supplies stored/full charge values;
Kirby's character-owned copied-charge hook preserves the unwitnessed guard.
The refactor retains branch, float-operation and RNG order at
ftCo_800B77E8, ftCo_800B630C, ftCo_800A96B8, ftCo_800ADC28,
ftCo_800B658C, ftCo_800BB768 and ftCo_800A0CB0. All 95 witnesses gate
exactly after moving these choices into the character tables.

`melee-lib/tests/allocation.rs` replays the recorded 6,001-tick
`corpus_v3_fd_fox_cpu9fox4_edb2b114a_p0` (seed 1, level-9 Fox in mode 4),
checks zero allocations in each step/observe and asserts that the measured
CPU both attacks and uses aerial Firefox recovery. This exercises the
Versus thinker, not just the pre-existing Nana path.

## Setting up a CPU player

- `make_boundary.py --cpu-levels 0 9` (one level per player, 0 for a person)
  writes each CPU's level into the match's start data at the creation hook
  the Game Start seed and spawn points already use (gm_801A4B08's store at
  0x8016E75C, before fn_8016D8AC copies the start data): PlayerInitData
  `slot_type` +1 = Gm_PKind_Cpu, `cpu_level` +0xF, `nametag` +0xA =
  GM_NAMETAG_NONE, as the CSS leaves a door switched to CPU
  (mncharsel.c:2476) with its slider set (mncharsel.c:2341). `cpu_kind` +0xE
  stays the CSS's 4 (gm_SetupPlayerDefaults, gm_1601.c:3509), so
  ftCo_800A101C gives the fighter CPU mode 4. The summary prints what the
  start data held before.
- Scenarios name a CPU fighter `controller = "cpu"` with `cpu_level = N`;
  `run_scenario.py` validates its traces like a scripted scene's (animation
  rates vary). The cold setup (`PlayerSetup::cpu_level`, `PlayerConfig::
  with_cpu_level` in the public API) and the savestate import (StaticPlayer
  `slot_type` +8, `cpu_level` +49, `cpu_type` +4A, stride 0xE90) make the
  fighter `PlayerKind::Cpu`. Recordings (`Config::cpu_levels`), boundaries
  (`boundaries.toml` `cpu_levels`), the explorer (a CPU's pad stays
  neutral) and `replay_to_scenario.py` carry the levels. The Slippi replay
  runner still refuses CPU players (a `"cpu"` controller without a level
  fails validation).
- `ftCo_800A101C` gives Donkey Kong and Bowser x56C 8.5 and Giga Bowser 17;
  `ftCo_800A0CB0` rerolls it every 600 ticks as 9, 18, 1 (Nana) or 4 times
  1 - r^3 by kind.

## The Versus think (CPU mode 4)

ftCo_800B2AFC sends modes 4, 11, 12 and every mode its switch does not list
to ftCo_800B24B8; mode 15 stands by. ftCo_800B24B8, in order:

1. ftCo_800A229C: the camera bounds (Camera_8003118C, `StageCamera::
   contains`); the hazard stages (Great Bay, Hyrule Castle, Brinstar,
   Rainbow Cruise, Big Blue, Icicle Mountain) fail closed. Off camera,
   ftCo_800AE7AC escapes; airborne it only rechooses its behaviour.
2. ftCo_800A5CE0 finds no star or hammer carrier without items.
3. ftCo_CpuInitNoTarget (ftCo_800A4BEC's nearest opponent), the item
   targets, then ftCo_CpuActOnNoTarget: ftCo_800A53DC's opponent (people
   first under xFA_b34; one player's two fighters rank alike) and
   ftCo_800A75DC's route to it (ftCo_800A4768's landing beside an island).
4. ftCo_800ADE48 chooses the behaviour; ftCo_800B2790 writes its script.

Behaviours ported for it: 2 (close attack: every kind's exclusions
ftCo_800B77E8, motion ranges ftCo_800B630C, follow-ups ftCo_800B658C
including Din's Fire ftCo_800B9504, the roll ftCo_800BA2E8, the edge guard
ftCo_800B732C), 3 (ranged attack ftCo_800B9CBC with ftCo_800B9A04 for an
item or article in hand, behaviour 3 ftCo_800B9020), 7 (defence
ftCo_800BB9B4 with ftCo_800BB104, 800BB768, 800BB220, behaviour 7
ftCo_800BA9A0), 9 (hold: DK's cargo walk in ftCo_800B683C,
`corpus_v3_fd_fox_cpu9donkeykong4_e9943b4ab_p2`), 10 (arrival with
ftCo_800BA674 and the neutral-special charge: DK 0x172, Samus 0x158 and
Mewtwo 0x156 are reached in the recordings), 4 (ftCo_800A96B8's Firefox
for Fox and Falco and the diagonal default, recorded for Fox, Falco,
Falcon, Peach and Marth), 1 (the revival platform's level-scaled wait
ftCo_800A08F0, Zelda's transformation in ftCo_800ABA34). The charge
release before every script (ftCo_800ADC28) reads a new
`CharacterCallbacks::NEUTRAL_CHARGE` hook.

## Words retail reads uninitialized

Two branches read registers or stack the code never wrote. The port models
what retail leaves there, and fails closed where it cannot know.

- **ftCo_800A4768 with no island end** (a target under the stage): it
  leaves ftCo_800A75DC's sp18 unwritten, which is ftCo_800B24B8's r1 -
  0x60 and - 0x5C. ftCo_800A4BEC's frame starts there, so x is its back
  chain, ftCo_800B24B8's r1 (0x804EE980 in every Versus CPU proc), and y
  the return address the last LR-saving call from ftCo_800A4BEC left
  (ftCo_IsAlly 0x800A4CB4, ftCo_800A0F00 0x800A4D48), unless
  ftCo_800A5F4C's item loop stored a distance there (800A6184). The CPU
  then heads for those bits as a destination (two denormals near the
  stage's centre). Retail: `corpus_v3_fd_fox_cpu9fox4_e89a89d0e_p0` tick
  553 and `corpus_v3_fd_fox_cpu5fox4_ef89b3e70_p0` tick 430, x54 =
  (0x804EE980, 0x800A4D48). `world::StaleStack` tracks the word.
- **ftCo_800AC5A0 with no knockback** (a pummel's hitlag): the stick is
  r5's and r30's low bytes. r30 is the Fighter (its address after boot is
  now known for a second player after a person's Fox: 0x80CB0980 on Final
  Destination, 0x80CA18A0 on Battlefield, `heap.rs`). r5 is what
  ftCo_800ADE48 left: seed_ptr (0x804D5F90) after HSD_Randf (the tumble
  switch, xF8_b5's roll), else mpCheckFloor's r1 + 0x44 from the
  destination probe (8004F188; a horizontal floor line takes
  mpLineIntersectionH and nothing writes r5 again), 0x804EE844 under
  ftCo_800B24B8 on Final Destination and Battlefield, whose floors are all
  horizontal. Retail: `corpus_v3_fd_fox_cpu5fox4_ef89b3e70_p1` tick 227
  (x 0x90) and `corpus_v3_bf_fox_cpu9fox4_e89a89d0e_p1` tick 252 (x 0x44).

## Unreachable here, with the evidence

- ftCo_800AB224's approach (xF9_b1) and ftCo_800ACD5C's ftCo_800BA674 arm
  need xF9_b1, which only ftCo_CpuSetTargetFlags sets, for a star or
  hammer carrier: items.
- CPU modes other than 4, 6 and 15 (ftCo_800B2AFC), ftCo_800ABA34's modes
  11, 12 and 16, and behaviours 11 and 12 (ftCo_800AD42C, 800AD54C): the
  single-player modes set them (ftCo_800A101C's x18 = 0xC for modes 1 and
  25, x20 = 0xB for 3); the Versus CSS leaves CpuKind 4.

## Every supported stage (wave 26)

Boundaries `w26_start_<stage>_fox_cpu9fox4` (a person's Fox against a
level-9 CPU Fox) on the twelve stages besides Final Destination and
Battlefield; explorer matches from each gate whole against retail. What
they reached:

- ftCo_800A1CC4 with ftCo_803C6594's Temple table (`stage_route.rs`,
  ft_3C61.c list0_*), ftCo_800AAF48's Temple island waypoints, and
  ftCo_800A9CB4_inline0's air dodge under the Temple's left arch and on
  Fourside. Great Bay's table stays guarded.
- ftCo_800A2C80's Fourside height (y < -35); Mushroom Kingdom's stays
  guarded.
- ftCo_800B89CC's Kongo Jungle barrel range (|x| <= 25) and behaviour 17,
  the barrel (ftCo_800AC434: stage_info.x73C, Ground_801C4DA0).
- ftCo_800AE7AC's grounded arm off camera (Flat Zone's camera is narrower
  than its floors): ftCo_800A6FC4's island points toward the blast zones'
  centre, one HSD_Randf per other island, else ftCo_800A75DC.
- ftCo_800AC5A0's r5 on any stage: mpCheckFloor leaves its own r1 + 0x44
  after a horizontal line and mpLineIntersection's 0/1 flag (li r5 at
  8004E98C/8004EAB8) after a sloped one (`melee_mp::SweepR5`), so the
  stale stick no longer needs all-horizontal floors.
- The Fighter address of a second fighter after a person's Fox on each
  stage, and of a CPU Zelda's Sheik on Final Destination (`heap.rs`, read
  from the boundary recordings).

Not CPU faults, left for their owners (the matches are not registered):
`w26cpu_ht_ef89b3e70_p2` tick 881 and `w26cpu_rec_luigi_ec3145eb3_p1`
tick 701, a hit effect's generator a little off where the person's Fox
enters DamageN2 from a special or a taunt under a jab (keys exact);
`w26cpu_bd_ef89b3e70_p0` tick 1259, the CPU's landing on Brinstar Depths'
rotating floor off by 2e-5; DownReflect out of DownDamage (fn_800C7DC4) and
Kongo Jungle's Klaptrap rider (Ground_801C1CD0) stop some explorer cases.
A wall tech in hitlag keeps x221A_b3 (`w26cpu_ht_ef89b3e70_p2` tick 829):
Fighter_ChangeMotionState never clears it.

## Recovery specials (wave 26)

ftCo_800A96B8's per-kind arms are profile data (`Recovery`): Quick Attack
(Pikachu, Pichu, ftCo_800A92CC), PK Thunder (Ness, ftCo_800A8EB0), Farore's
Wind (Zelda, ftCo_800A949C), the Green Missile (Luigi), none (Yoshi), and
the diagonal (Samus, ftCo_800A963C writes the same script). Witnessed by
`w26cpu_rec_<kind>_*` (explorer cases from `start_fd_fox_cpu9<kind>4`).

## Wave 27's stages

Boundaries `w27s_start_<stage>_fox_cpu9fox4` (a person's Fox against a
level-9 CPU Fox) on Mushroom Kingdom II, Great Bay, Icicle Mountain,
Brinstar, Venom, Mute City and Poké Floats, warm and cold; explorer
matches from each gate whole (`corpus_v3_w27s_start_*`). Ported:

- ftCo_800A229C's stage arms: Great Bay's turtle away from its rest
  (grGreatBay_801F66A4, map 1's xC4, right of x = -5), Brinstar's acid
  above the bottom blast zone (Ground_801C4368: always, so a Brinstar CPU
  escapes every think and keeps its attack modes), Icicle Mountain within
  `0.4 * |scroll| + 0.4` of the blast zone the stage scrolls toward.
- ftCo_800A8210 for them: grounded, the farthest safe island end from the
  danger (ftCo_800A6700); on Icicle Mountain the nearest platform above
  (ftCo_800A6A98) or island below (ftCo_800A6D2C, a Randf per island).
- ftCo_800A2718's Brinstar islands (an end the acid covers within 90
  frames, ftCo_800A1F98) and Great Bay's route table (ft_3C61.c list1_*).
- grVenom_80206D10 in ftCo_800A1B38: Venom's floors on joints 3 and 4 are
  ignored; a CPU recovering onto one jumps off it (ftCo_800B2790 case 4,
  ftCo_800A0148).
- The Fighter address of a CPU after a person's Fox on each of the seven.
- A savestate import keeps the island lists in retail's saved order
  (`initial_state/islands.rs`): Brinstar's five joint restores each ran
  mpIsland_8005B334, which reverses the static lists, and the CPU's
  recovery took the first island of the reversed list.

Mushroom Kingdom II, Mute City and Poké Floats need nothing of their own.
`CPU_SUPPORTED` gains Mushroom Kingdom II, Great Bay, Brinstar, Mute City
and Poké Floats. Icicle Mountain and Venom stay out: their boundaries gate
exact, but `corpus_v3_w27s_start_im_fox_cpu9fox4_ef89b3e70_p0` diverges at
tick 1197 (p1.cur_pos.y) and `..._venom_..._edb2b114a_p0` at tick 4238,
not yet triaged (docs/handoffs/w27-stage-cpus.md).
Not CPU faults, left for the owner: a reflected Arwing laser
(itarwinglaser.c:600) stops one Venom explorer case.

## Still fail-closed

Kirby's mouthful in
ftCo_800B683C; a shrunk Yoshi's jump (ftCo_800A0508); Nana alone
(ftCo_800AEFB8, the Versus think from ftCo_800B101C's frame, r1
0x804EE930).

Items (work items 10-18): picking up (ftCo_800A3710), heading for an item
(ftCo_800A866C), a swing weapon's list (ftCo_800B52AC), a Bob-omb's fuse
(it_8026B960) and a shooter's ammunition (it_8026B924). Stages (work item
2): the hazard stages, routes and moving floors listed in PARITY. Two
players' opponents ranked by gm_8016C6C0 (three or more players).
ftCo_800AC5A0's r5 after an unknown caller.


### Persistent rapid-jab counter and incomplete warm creation

The recorded Falco CPU case `corpus_v3_fd_fox_cpu9falco4_edb2b114a_p0`
exposed Fighter+x1A54 surviving Dash into jab2 at tick 2919. Retail
`ftCo_Attack_800D6A50` increments that persistent word (800D6A80) to 4 at tick 2931,
entering Attack100Start. It is reset by fresh jab1 (8008AC80), independent
of the per-move union. The port now owns it in FighterCore and imports its
saved value rather than discarding it with JabState.

The warm `start_fd_iceclimbers_cpu9fox4` savestate stops during the primary
CPU Fox's construction: Entry 322, no animation, script PC 0x5acb5acb and
uninitialized callback words 0xe6a8a8e6/0xe6e6e696/0x8da8a9e6. Its later
frame-zero dump has completed initialization. The importer explicitly
rejects that unfinished saved construction and directs callers to the
verified cold twin; it does not borrow the later trace row.

This save is not in `harness/boundaries.toml`: registered explorer boundaries
must also pass the warm/cold initial-state comparison. The standalone warm
scenario remains a rejection regression and its cold twin remains in the
95 strict CPU witnesses. A properly completed save is needed before restoring
named explorer access and automatic boundary selection for this setup.


Initial extra reach (ftCo_800A101C, retail 800A1774 onward) also comes
from the static CPU profile: 3.5 by default, 8.5 for Donkey Kong and Bowser.
CpuSetup carries that number rather than a character kind. Nana retains the
existing semantic partner override of 1.0. Giga Bowser has no registered
CharacterTable; a future table must supply retail's 17.0 explicitly.

The queued transformation eligibility in idle/arrival behavior is also a
character-owned profile capability, enabled only by Zelda and Sheik. Shared
CPU code consumes the request without classifying the fighter's kind.
