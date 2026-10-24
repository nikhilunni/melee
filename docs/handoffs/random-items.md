> **Branch** `worktree-agent-adfc6624be44a0fcf` (tip `e703ae25`), not merged into main. Based on 26fee4c7; to merge, rebase onto main, then run the full sweep and replay-batch over public-v3.7 (every new item is grabbable at creation, which reaches replays). All commits are verified; none is WIP.

# Handoff: random items (Item Switch on)

Branch `worktree-agent-adfc6624be44a0fcf`, rebased onto main 26fee4c7.

## Task and scope

Wave 13 (breadth). Build the foundation for matches with random items on:
1. Boundaries with the Item Switch (frequency, which kinds) in `make_boundary.py`,
   carried by the cold twin, the recording format and `melee-lib` setup.
2. Port retail's random item spawning (itspawn.c) bit-exact with its RNG draws.
3. The containers (Capsule, Box, Barrel, Party Ball, Egg), then simple items.
Witnesses: FD boundaries with items at high frequency, explorer runs from them,
registered in `m5_gate`. Unported items stay `unimplemented!` by name.

## Done and verified (all commits are verified; none is WIP)

Details in `docs/PORT_NOTES/RANDOM_ITEMS.md`; reachability in
`docs/COVERAGE_AUDIT.md` ("Random items").

- Harness: `make_boundary.py --item-frequency very-high --items Sword ...
  [--no-containers | --containers Taru]`; `boundary_script.py` pokes
  0x8045C370 / 0x8045C378 and replaces StartMeleeRules.x20 at the creation
  hook; scenarios/registry/recordings carry `item_frequency` + `item_mask`
  (hex); `replay_to_scenario.py` matches boundaries by them. Tests in
  `harness/tests/test_boundary.py`.
- Port: `melee-it/src/random_spawn.rs` (ItemRules, PickTable, RandomItemSpawner),
  stage weights `melee-gr::desc::read_item_rates`, cold setup draws the first
  interval BEFORE the stage's creation draws (`initial_state/cold.rs`), warm
  import reads it_804A0E30 (`scene_flow::restore_item_spawner`), scheduler row
  `Callback::ItemSpawner` (GObj 5/7, s_link 0) and `Runtime::spawn_random_item`
  in `melee-lib/src/frame.rs`; efAsync 0x420 puff. New items are grabbable at
  creation (it_8026B390 in Item_80267130).
- Containers, `crates/it-container`: Capsule (all but sliding), Box (rest, fall,
  break into items with efAsync 0x427 boards in `melee-ef/src/box_fragments.rs`),
  Barrel (fall/bounce, rest, roll, struck roll with striker as owner, break
  open). Scene release: `ItemEvent::Release` / `CrateContents`, `Resume`
  (`frame.rs::release_contents`). Heavy items push apart (it_80271F78).
- Witnesses (`RANDOM_ITEM_WITNESSES` in `crates/melee-sim/tests/m5_gate.rs`),
  all exact with particles: start_fd_items_sword_nocont(+_cold),
  start_fd_items_sword(+_cold), start_fd_items_taru(+_cold),
  start_bf_items_sword(+_cold), corpus_v3_fd_items_sword_nocont_{ef89b3e70_p0,
  e005a4f43_p2, e266e1150_p0, eecfcf1ea_p0}, corpus_v3_bf_items_sword_{e32b8fdc4_p1,
  e7d19908f_p1, e58dc731e_p2}, corpus_v3_fd_items_sword_{ee8d2a62f_p0,
  e9af22990_p0, efc83a559_p1}, corpus_v3_fd_items_taru_ece1e43b7_p2.

## Where I stopped

A full sweep against `target/sweeps/26fee4c7.tsv` was started (output
`target/after.tsv` in the worktree) but not read; filtered sweeps (start_,
peach, sd, items) showed 0 regressions. replay-batch was NOT run; the change
"every item grabbable at creation" reaches replays.

## Next steps

1. Run the full sweep with `--lock .../target/sweep.lock` and replay-batch over
   `~/melee-data/replays/public-v3.7` (no replay may match fewer frames).
2. Box/Barrel broken empty: port the blast hitbox state (retail hitbox0.state 2,
   port 1 on the blast's first tick; corpus_v3_fd_items_sword_e7254fba4_p2,
   corpus_v3_fd_items_taru_ee6ff560c_p0). Compare with the Capsule's blast,
   which is exact (Capsule uses Item_80268E5C(.., 5, 2), Box state 6 with 2).
3. Fighter heavy carry: HeavyGet, ftCo_Lift.c (LiftWait/Walk1/Walk2/Turn),
   HeavyThrow states in ftCo_ItemThrow.c — the most common explorer fault from
   container boundaries (`item_pickup.rs` `try_item_pickup`).
4. Beam Sword battering use: smashes/tilts/aerials/C-stick smash with a held
   kind-2 item (`melee-ft/src/fighter/attack.rs`, `item_pickup.rs` HELD_ITEM_STATES).
5. Party Ball (itkusudama.c): my port was withdrawn (see commit 1bcb1107 for the
   full code, removed in b6a56d35). Its settling landing's bounce spark in retail
   is placed without the pitch it_8028A190 sets (start_fd_items_kusudama, tick 495).

## Commands

```sh
cargo build --release -p melee-sim
./target/release/melee-sim gate /Users/nikhilunni/Projects/melee/harness/scenarios/start_fd_items_sword_cold.toml
./target/release/melee-sim sweep --scenarios /Users/nikhilunni/Projects/melee/harness/scenarios --filter items
cd harness && MELEE_DATA_ROOT=/Users/nikhilunni/Projects/melee uv run python make_boundary.py \
  --stage FinalDestination --players Fox Marth --name <n> --item-frequency very-high --items Sword --no-register
cd harness && MELEE_DATA_ROOT=/Users/nikhilunni/Projects/melee uv run python explore_batch.py <out> 64 0 \
  --boundary start_fd_items_sword --boundary start_fd_items_taru --samples 10
```

## Boundaries and recordings

Savestates in the main checkout's `harness/roms/` (start_fd_items_sword_nocont,
start_fd_items_sword, start_fd_items_taru, start_bf_items_sword,
start_fd_items_kusudama — the last unregistered); traces in its `harness/traces/`.
Registry entries in this branch's `harness/boundaries.toml`. Explorer outputs:
`target/explore_{sword1,cont1,cont2,cont3}/summary.md` in the worktree.

## Gotchas

- Only bash-simple commands pass the worktree guard; write edit scripts to
  files instead of heredocs.
- The setup draw of the spawner precedes stage creation draws (verified FD, BF);
  other stages unverified.
- Party Ball scripts use item opcodes 24/25 (colour flash); loading its assets
  panicked before that decode was added (also withdrawn).
- `command return without call` panic in the item script engine on a barrel
  dropped off a roll (corpus_v3_fd_items_taru_e17844051_p1,
  corpus_v3_bf_items_sword_e5993e1bc_p2) — open.

## Divergences outside my area

None found; all non-item explorer faults were fail-closed fighter sites
(HeavyGet, sword attacks).
