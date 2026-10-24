> **Branch** `worktree-agent-a6ee26077e1ef8c8e` (tip `3e4b9835`), not merged into main. Based on 26fee4c7; verified there (full sweep 0 regressions, 91 fixed; replay-batch 108/108). To merge, rebase onto main (first conflict: harness/make_boundary.py and harness/dolphin/boundary_script.py against the all-stages work, then m5_gate and boundaries.toml), re-gate CPU_WITNESSES, sweep and replay-batch. None of the 24 commits is WIP.

# Handoff: CPU players in a Versus match (wave 13, 2026-10-24)

Branch `worktree-agent-a6ee26077e1ef8c8e`, rebased on main 26fee4c7.
Details of the port itself: `docs/PORT_NOTES/CPU_PLAYERS.md`.

## Task and scope

Work items 1, 3 and 4 of `docs/PARITY.md`: make a CPU player in a VS match
bit-exact. (1) boundaries with CPU ports and a match setup that accepts a CPU;
(2) port the CPU's mode dispatch and behaviour table, attack choice and
recovery/tumble (`crates/melee-cpu`); (3) witnesses on FD/Battlefield (human
Fox vs CPU Fox at levels 1/5/9, CPU Marth and others) and explorer runs,
registered in `m5_gate`. Out of scope: work item 2 (other stages' awareness,
routes, hazards) and item-related CPU branches.

## Done and verified (no WIP commits; every commit gates exact)

- Setup: `make_boundary.py --cpu-levels 0 9` (start-data poke at the
  creation hook), scenario `controller = "cpu"` + `cpu_level`, cold setup,
  savestate import (StaticPlayer +8/+49/+4A), public `PlayerConfig::
  with_cpu_level`, recordings/boundaries/explorer/`replay_to_scenario.py`
  carry levels. Replay runner still refuses CPU players.
- melee-cpu: mode 4 think (ftCo_800B24B8), danger/camera escape (air),
  targets (800A53DC, 800A75DC, 800A4768, 800A4E8C), defence (behaviour 7),
  ranged (behaviour 3, held item 800B9A04), close attack per-kind
  (exclusions, follow-ups, Din's Fire, roll, edge guard), arrival + charges
  (DK/Samus/Mewtwo), hold incl. DK cargo walk, recovery Firefox + diagonal
  default, revival wait, Zelda transform, x56C by kind, stale-stack/register
  models (800A4768 sp18, 800AC5A0 r5/r30; `world::StaleStack`, `heap.rs`).
- Witnesses: `CPU_WITNESSES` in `crates/melee-sim/tests/m5_gate.rs`
  (94 scenarios): all `start_*_cpu*` start scenes and `_cold` twins (FD:
  Fox vs every kind's CPU9, Fox CPU1/CPU5, Marth CPU5; BF: Fox CPU9, Marth
  CPU9, Sheik vs Fox CPU9; CPU on port 1 `start_fd_cpu9marth_fox4`, both CPU
  `start_fd_cpu9fox_cpu9marth4`, `start_bf_cpu7falco_cpu3peach4`), and the
  explorer cases `corpus_v3_*_cpu*` (e.g. `corpus_v3_fd_fox_cpu9fox4_e89a89d0e_p0`
  stale sp18, `corpus_v3_bf_fox_cpu9fox4_e89a89d0e_p1` r5 0x44,
  `corpus_v3_fd_fox_cpu9donkeykong4_e9943b4ab_p2` DK cargo, 6000-tick
  samples for Fox, Falcon, Samus, Roy, Pichu).
- Full sweep vs `target/sweeps/26fee4c7.tsv`: 0 regressions, 91 fixed.
  replay-batch public-v3.7: 108 complete, same as main's binary.

## Where I stopped / next steps

1. `corpus_v3_fd_fox_cpu9falco4_edb2b114a_p0` (recorded explorer sample)
   diverges at tick 2931: triage it (`melee-sim triage`), not registered.
2. `start_fd_iceclimbers_cpu9fox4` (warm savestate import) panics at
   `crates/melee-lib/src/initial_state/fighter.rs:131` (command instruction
   offset not found for a fighter mid-script at the boundary); its cold twin
   passes and is registered. Probably outside melee-cpu.
3. Two whole CPU-vs-CPU matches (`cpu_match_fd_cpu9fox_cpu9marth4`,
   `cpu_match_bf_cpu7falco_cpu3peach4`, frames 20000; TOMLs only in the main
   checkout's `harness/scenarios`, not committed) failed to record: the
   recorder timed out (`--timeout 600`). Record with a longer timeout
   (`record.py ... --timeout 3000` via record_many args) and register.
4. Re-port the branches put back to `unimplemented!` once a recording
   reaches them (asm reads are in commits a27d35dd, 841585a2, e9c52f6d,
   22a7eaae): ftCo_800A96B8 arms for Pikachu/Pichu (800A92CC), Yoshi, Ness
   (800A8EB0), Luigi, Zelda (800A949C), Samus (800A963C); Kirby's mouthful
   (800B683C); shrunk Yoshi jump (800A0508); Nana alone (800AEFB8 = the
   Versus think from frame r1 0x804EE930); a CPU Kirby's copied charges.
   Directed witnesses: make boundaries with those CPUs and explore.
5. More explorer seeds: `target/scratch`-style command below with new
   seeds; the bridged faults give witnesses for the next sites.

## Commands

```sh
CARGO_INCREMENTAL=0 cargo build --release -p melee-sim
MELEE_DATA_ROOT=/Users/nikhilunni/Projects/melee ./target/release/melee-sim gate /Users/nikhilunni/Projects/melee/harness/scenarios/<name>.toml
cd harness && MELEE_DATA_ROOT=/Users/nikhilunni/Projects/melee uv run python make_boundary.py --stage FinalDestination --players Fox Marth --cpu-levels 0 9 --name <n> --no-register
cd harness && MELEE_DATA_ROOT=/Users/nikhilunni/Projects/melee uv run python explore_batch.py ../target/explore/<out> 2 5 --boundary start_fd_fox_cpu9fox4 ... --per-fault 2 --samples 8
cargo test --release -p melee-sim --test m5_gate cpu_players   # main checkout only
```

## Boundaries and recordings

35 CPU entries appended to `harness/boundaries.toml` (`cpu_levels = [..]`).
Savestates `harness/roms/start_*_cpu*.sav`, traces `harness/traces/` (main
checkout), scenario TOMLs committed under `harness/scenarios/`.

## Gotchas

- Retail reads uninitialized words in two CPU branches; the port models them
  from the stack frame (0x804EE980 = ftCo_800B24B8's r1) and fails closed
  for unknown callers or stages with sloped floors (`StaleStack::after_floor_probe`
  is FD/BF only). The explorer's `Match::new` needs Fighter addresses after
  boot (`heap.rs` table, recording-derived).
- `ftCo_800ADE48`'s tumble switch uses an unset r31: always true under mode
  4 (r31 = fp+0x1A88 or the escape kind).
- Defence helpers/rolls live in `roll.rs`; follow-ups in `follow_up.rs`
  are shared by behaviours 2 and 3.
- Single-player modes, hazard stages and item branches remain fail-closed
  with evidence in the port notes (camera bounds contain every FD/BF floor:
  800A6FC4 unreachable there).
