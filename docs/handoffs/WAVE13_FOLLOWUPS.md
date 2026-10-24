# Wave 13 follow-ups

Collected from the wave-13 agents' reports (2026-10-24): divergences found, sites still open, unwitnessed ports. Each is a candidate task for the next wave; the per-branch handoff notes in this directory hold the rest.

- efLib_Cb_PtclAppSRTHook: user functions on bank-0 generators 0x96/0x97/0x98/0x21B, no port found (item 27 agent). Check reachability / port.
- melee-ft commands.rs:899: two efAsync flushes in one proc with a sound draw between; 283 rows could set it up; search for a witness.
- Kirby copied Rollout after cape (fn_80105978): needs a three-fighter cold setup (start_fd_mario_kirby_jigglypuff4 recorded, unregistered).
- Peach doAirEnd0 (special_s.rs:427): Bomber inert hitbox touches shield + item hurtbox same tick; try with turnips.
- Yoshi rows 348/350/353/355: Egg Lay on container monsters (needs random items).
- Jigglypuff Stadium sleep: event matches only.
- Effects default arms (dust.rs:149, egg_shell.rs:137, item_generators.rs:41/:133, fighter/effects.rs:328, lib.rs:314) close as items/stages land.
- melee-cm standard.rs:527, Battlefield light kinds, animated light descriptors: with single-player / other-stage bring-up.
- hsd-particle Error::OutsideRetailData sites are not counted by parity_inventory regex any more.
- corpus_v3_ys_ganondorf_fox4_e281b1a7a_p1 (untracked, new): particle generator id 723 vs 724, bank 0 program 225 (seen on 21a686c7 sweep)
- Samus's own muzzle flash model 0x7D1 loaded async in melee-ef though efAlt_Spawn sets sync (charge agent); no witness yet.
- DK Giant Punch hitbox-1 check in ft-donkey-family giant_punch.rs stays unimplemented! for Kirby's copy (PlKbCpDk.dat not scanned); scan it and make unreachable like PlDk.dat.
- corpus_v3_fd_kirby_captainfalcon4_e1cda1301_p1 / _e2798fef9_p2 (recorded): panic grab_pairs.rs:215 "constrained without a partner" (thrown_pose set, no grab link). Undiagnosed.
- CaptureCut.c:137-189 (fighter dying pinned in Kirby's mouth): witness kirby_inhale_randall_pit_ys_fox4 tick 1358; WIP patch target/wave13/pinned_death_wip.patch; Fox placed 1.53 right / 0.34 up of retail's XRotN release.
- corpus_v3_ys_kirby_fox4_e45211ed5_p0 diverges tick 642: Fox hits Kirby's Stone while Kirby holds Fox's copy; retail full hit on shell (x1834 38->20.43, no hitlag), port treats as phantom. bones-diff shows 4-index shift for Kirby (dump mapping problem?).
- Kirby still open (searched, none): airborne throw landing before flag (fn_800DD5EC), CaptureCut.c:29-30, pull/gulp losing floor, cutter beam clank/absorb, CaptureCut/Throw/ThrownKirby rarer partings (need ditto/third fighter), wall-bounce star, bump-star cooldown.
- Ported for Kirby via shared generics but not witnessed on Kirby: Toad grounded rows leaving floor (8010C8D8), item caught by Kirby's Toad; ability star ending on a ceiling (same C condition as the witnessed wall).
- Explorer crash: melee-ft commands.rs:805 "no entry found for key" via remove_part_animation <- release_hand_pose (item_pickup.rs:505) <- apply_pending_pickup_pose (v3-fd_peach_captainfalcon4 exploreb32e3881 p1 t1577; v3-bf_mewtwo_peach4 explore0ca2b8e1 p0 t2794).
- Explorer faults: holding an item in AttackAir{Lw,Hi,F} not audited (fd_peach_fox4, dl_donkeykong_peach4); ftData_OnItemPickupExt for this kind (bf_mewtwo_peach4); "mv+4 written from outside during ActionId(18)" (fd_pikachu_fox4).
- Item agent still open: it-yoshistar:84 (stars off shield, ~2000 candidates none), map.rs:744 wall pinch, it-bombhei/it-dosei stage/random-spawn states (need random items).
- Recording determinism under heavy host load: 124/1472 coverage runs and the plain headless build differ at record 4 ps_frame under load 200-290. Re-record witnesses on a quiet machine; keep record_many --jobs modest when agents run.
- Retail coverage: 462 explorer corpus scenes still unmeasured (retail_coverage.py --m5 --resume); decide whether to apply Dolphin patch 0004 to the main headless build.
- mv+4 (inherited scratch word) still wrong (not missing) where special rows lack a RETAINED_SCRATCH_WORD arm: Fox Fire Fox 353-356 angle reaches CliffCatch and Squat 39. Use `melee-sim scratch-diff --word`. Open: landing.rs:362 (Zelda/Sheik Transform arrival), landing.rs:350 + pikachu special_lw.rs:396 (Thunder lead's outside write).
- spawn.rs:867 change_motion_state flags SkipThrowException, KeepSwordTrail, FreezeState still fail closed.
- start_fd_kirby_iceclimbers4 panics in initial_state/fighter.rs:131 on import (from the Zelda/Bowser/IC copy agent; check at its merge).
- N64 stages open: BarrelWait death (ftCo_8009EAF8), barrel catch holding heavy item / in grab pair; savestate importer refuses fading cloud / visible guest / held fighter / flying bird; Slippi neutral-spawn rows for N64 stages; 24 Versus stages named but unported.
- Match flow: team stock steal on Start (gm_16AE.c:576-586) fails closed; four doubles replays stop around a steal (FALCO 18_57_46 diverges at 7803). Two doubles replays now match in full but are not registered as Slippi witnesses.
- CPU: `corpus_v3_fd_fox_cpu9falco4_edb2b114a_p0` diverges at tick 2931 (untriaged); warm `start_fd_iceclimbers_cpu9fox4` panics at initial_state/fighter.rs:131 (the Zelda/Bowser/IC branch has a fix for this import); `cpu_match_*` recordings need a longer recorder timeout.
- `corpus_v3_ps_bowser_ness4_edb5ae9dd_p0` diverges at tick 924: retail creates a Ness item (kind 102) the port does not.
- `kirby_stone_phantom_fd_fox4`: port and retail disagree on phantom vs ordinary hit for a shallow contact (WIP commit on the grab branch).
- Four-Ice-Climbers explorer case (seed 89a89d0e, profile 2) stops at melee-cpu tumble.rs:100 (ftCo_800AC5A0).
- Random items: Box/Barrel broken-empty blast hitbox state 1 vs retail 2; Party Ball landing spark pitch; item script "command return without call" with a barrel dropping off a roll; HeavyGet/Lift/heavy throws and Beam Sword attacks are the bulk of explorer faults.
- Kongo Jungle: the moving barrel's position is a step behind retail's; Klaptrap event (grKongo_801D7BBC) unported.
