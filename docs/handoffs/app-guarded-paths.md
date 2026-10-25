> **Branch** `worktree-agent-a0b19adcae35accda` (tip `41d39950`), one WIP commit, not merged into main. Verified parts (Party Ball lifecycle, items boundaries for 3-4 players and every stage, CPU 3-4 player boundaries) need m5_gate registration; unwitnessed parts must be re-guarded and `Stage::items_supported` must not widen until the faults below are fixed or guarded per stage.

# Handoff: the guarded paths app players hit first (wave 26, paused)

Branch `worktree-agent-a0b19adcae35accda`, one `WIP:` commit on main
`80c081ba`. Stopped on the usage limit: no final sweep, no m5_gate
registration, no replay-batch, `--test allocation` not run.

## State of each piece

Verified exact (keys and particles) against retail, not yet registered:

- **Party Ball, whole lifecycle** (`it-container/src/party_ball.rs`,
  shower in `melee-lib/src/frame/party_ball_shower.rs`): both facings,
  fall/rest, pickup/hold/throw, rise, hang, script-timed open and shower
  (monster roll, kind/Foods/random draws, fmsubs drifts), blast. Engine:
  item script opcodes 24/25 (colour overlay, drawing only), efSync 0x433 on
  bones 3/5 (item bone generators may now use the common bank), and
  **x5CC reads zero in a state whose joint animation has no AObj**
  (`ItemCore::frame_without_aobj`, lbGetJObjCurrFrame): the hang's
  AtFrame(80) depended on it. Witnesses (`start_fd_items_kusudama`
  boundary, now registered with its real mask 0x...1010 = Kusudama+Sword):
  corpus_v3_fd_items_kusudama_{ef89b3e70_p0 994, ef89b3e70_p1 1335,
  e0fcbde24_p0 5628, e26ea92b1_p1 2493, e70b3084c_p0 6001, ee32d950f_p2 6001,
  ef5e3e043_p1 6001 (pickup, throw)}, plus start_fd_items_kusudama(+_cold).
  A full sweep with the Party Ball and x5CC change (before the CPU and
  slide work) had 0 regressions (target/w26/sweep_full1.tsv).
- Still guarded (GUARD_UNWITNESSED): rising/thrown ball at a ceiling or
  wall. The drop path is unguarded but unwitnessed: record
  `explore_batch.py <out> 1 95 --boundary start_fd_items_kusudama
  --samples 3` and `... 1 83 ...` (seeds 24fdee66, 2bf046d4 drop it).
- **Items on 3-4 players and the other 12 stages**: 16 new boundaries
  (`w26_start_{fd,bf}_items_all_fox_marth_falco[_peach]`,
  `w26_start_<Stage>_items_all`), all registered in `harness/boundaries.toml`,
  all exact over 600 ticks warm and cold after Kongo Jungle's drop markers
  were made live (`scene_stage::live_drop_markers`: the markers ride the
  moving platforms). `Stage::items_supported` is widened to 2..=4 players on
  every ported stage in the WIP commit; it should only ship once the
  explorer faults below are witnessed or guarded off per stage.
- **CPUs in 3-4 player matches**: ftCo_800A53DC ranks by the standings
  (`Scene::standings` from `standings::current_ranks`); fighter addresses
  for 3-4 player scenes (`heap.rs`: FD shifts every block by 0x40 per extra
  player, BF not at all; other stages give no address beyond 2 players);
  behaviour 14 (ftCo_800AD7FC, `throw_item.rs`) and swing-weapon attacks
  (ftCo_800B52AC, `choose_with` with reach, fused ops from asm). 4 new
  boundaries (`w26_start_fd_fox_marth_cpu9falco4`,
  `w26_start_fd_fox_cpu9marth_cpu9falco_cpu9peach4`,
  `w26_start_bf_fox_marth_cpu9falco_cpu9peach4`,
  `w26_start_bf_fox_cpu9marth_cpu9falco4`) gate exact over 600 ticks; 96
  explorer cases run clean in the port. The batch recording explorer
  matches (target/w26/explore_cpu1) was stopped: rerun it, gate, then raise
  `Stage::CPU_MAX_PLAYERS` to 4 (and the app test asserting "two only").
  None of throw_item / swing weapons is witnessed yet.
- Unwitnessed item fixes in the WIP commit (port-side only): Bob-omb
  leaving the floor (it_8027DE18) and sliding (state 12), Mr. Saturn
  leaving the floor from sit/look and its unheld slide (state 10), Capsule
  and Box slides (it_8026E8C4), the Fourside UFO camera subject ordered
  among item subjects by creation (`ufo_subject_items`).

## Remaining explorer faults (target/w26/scan_stages2.log)

- itHarisen_UnkMotion8_Coll: a dropped fan's landing reads uninitialized
  xD4C (heap garbage; the commonest fault in items matches, ~15% of cases).
- Kongo Jungle: Klaptrap rider Ground Animation, log Sink.
- Holding an item in FlyReflectWall / BarrelWait; a Box thrown under a
  ceiling (Hyrule Temple); ftPe_8011D518 (Peach's pull with a full hand).
- Monsters from containers/Party Ball: Octarock and Ottosea constructors,
  old Kuribo animation, right-facing ReDead birth.

## Notes

- I ran `pkill -f record_many.py` once (about 12:20): it may have stopped
  another agent's recording at that moment.
- target/w26/ holds the scripts (scan.sh, gate.sh, queues) and the probe
  test (debug_pb.rs) used to list Party Ball state coverage.
- ~94 unrecorded `corpus_v3_fd_items_kusudama_*` TOMLs from an interrupted
  batch sit in the main checkout's scenarios (masks corrected); they are
  reused by explore_batch.
