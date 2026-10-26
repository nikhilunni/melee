# Handoff: Hammer, Super/Poison Mushrooms, Bunny Hood pool (wave 27)

Branch `worktree-agent-a04cc103feaee3dd4`, one `WIP:` commit. The port of
the Hammer (item, head, fighter Hammer rows 307-313) and the Mushrooms
(it-kinoko, fighter size change rows 314-321) is complete as far as the
explorer reaches, but the wave's acceptance was not run: no full sweep
against the baseline, no replay-batch, no witnesses registered, nothing
added to `PORTED_RANDOM_ITEMS`.

## What gates exact (worktree melee-sim, main-checkout recordings)

- Directed: `w27_hammer_swings` (1100 ticks), `w27_kinoko_idle`,
  `w27_dkinoko_idle` (1800 ticks), all with particles.
- Hammer explorer bridges: `corpus_v3_fd_items_hammer_*` e005a4f43_p1,
  e00f31913_p0/p1, e0d368f02_p2, e0fcf0c70_p2, e3e74affa_p1, e41b2c2b8_p2,
  e50f774a0_p1, e573e2d95_p0, e6cc80d32_p0, e75fb4a9a_p0/p1 (head loss),
  e89a89d0e_p0, e89f997dc_p1, eb8786a38_p0, ec13743d5_p0 (air catch),
  ec48387e0_p0, ec513cd66_p2, e8f0de8c6_p0, ed486ea18_p2.
- Mushrooms: `corpus_v3_fd_items_kinoko_*` e19f8579b_p2, e50f475b4_p0,
  e50f475b4_p1 (grab parting), e726cfdde_p1, ec13743d5_p1.
- Bunny Hood: explorer 288 cases, 0 faults; rabbitc ef89b3e70, e50814092,
  e4213e6a5, ef9b6d16d (_p0) exact.
- Allocation test `hammer_and_mushrooms_step_and_capture_allocate_nothing`
  added (crates/melee-lib/tests/allocation.rs); last run before the final
  two fixes.

## Open

1. `corpus_v3_fd_items_hammer_ef9e858d4_p2` diverges at tick 2082: the
   launched head's vel.x/vel.y off by 1-2 ulp (HammerLanding, head loss).
   Suspect the head/handle joint matrices (`held_hammer_request` in
   crates/melee-lib/src/frame.rs, `poses.bone_matrix` read tick) or
   `lbVector_Normalize` vs `melee_lb::vector::normalize`; compare with
   it_80293E34's asm (`asm.py it_80293E34 --fused`) and a `--bones` recording.
2. Two last fixes are unwitnessed: `grab_escape::let_go` now sets down a
   thrown victim (ftCo_800DC920 x2226_b2 path) when a size change parts a
   throw (scenario `corpus_v3_fd_items_kinoko_edb2b114a_p0`), and
   FlyReflectWall/Ceil are audited for the Hammer (DamageFly's callbacks;
   scenario `corpus_v3_fd_items_hammer_e24fdee66_p1`). Both were queued in
   a record_many batch (log target/w27/rec3.log in the worktree) with
   `w27_hammer_drop`, rabbitc ece1e43b7_p0 / e46703f61_p0 and hammer
   ee133b82f_p2 / e50f475b4_p2; gate them when recorded.
3. Re-gate the kinoko1 explorer bridges (target/w27/kinoko1/summary.md) with
   the current build; dkinoko explorer (48 seeds) ran port-only with 0
   faults: bridge a few samples and record.
4. Register witnesses in m5_gate.rs (RANDOM_ITEM_KIND_WITNESSES); copy
   the corpus TOMLs into harness/scenarios; add RabbitC, Hammer, Kinoko,
   DKinoko to PORTED_RANDOM_ITEMS (crates/melee-lib/src/config.rs).
5. Full sweep vs target/sweeps/e7e3a37d.tsv (or the new main's), replay-batch
   over public-v3.7 (shared fighter code changed: scale, attribute_scale,
   begin_animation_phase order, change_motion_state redirects), clippy,
   then split into per-item commits if feasible and rebase onto main
   (expect conflicts in HELD_ITEM_STATES, PORTED_RANDOM_ITEMS, boundaries.toml).

## Guarded (unimplemented! with C line)

- fn_800D290C: a grab pair in a Kinoko state (mushroom.rs).
- Others listed in docs/PORT_NOTES/RANDOM_ITEM_KINDS.md (Hammer and
  Mushrooms sections).
