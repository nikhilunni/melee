# Lane C: Peach idle, match start and movement

Peach vs idle Fox on Final Destination passes all eighteen recorded scene gates
and their ordered particle RNG ledgers. The character is kind 9 in `ft-peach`;
registration is one line in `scene_fighter.rs`. The start and idle bone oracles
compare all 114 Peach joints (45 dynamic) and 73 Fox joints at tick boundaries.

The recorded start transitions are 0:322, 6:323, 35:324, 65:29, 81:42,
111:14. The forward double jump enters 27 at tick 114 and FallAerial (32) at
174; the backward double jump enters 28 at 52 and FallAerial at 112.

## Commands and final lines

Run from the repository root. All listed commands used the existing local
recordings and disc files; no oracle expectations or scenario inputs changed.
The eighteen CLI commands and their complete final lines were:

```text
$ cargo run -q -p melee-sim -- gate harness/scenarios/idle_fd_peach.toml
600 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_peach.toml
600 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/squat_fd_peach.toml
300 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/turn_fd_peach.toml
300 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/walk_fd_peach.toml
300 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/walkfast_fd_peach.toml
300 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/dash_fd_peach.toml
300 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/turnrun_fd_peach.toml
300 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/jump_fd_peach.toml
300 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/airjumpb_fd_peach.toml
300 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/shield_fd_peach.toml
300 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/spotdodge_fd_peach.toml
300 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/roll_fd_peach.toml
300 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/airdodge_fd_peach.toml
300 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/wavedash_fd_peach.toml
300 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/ledge_fd_peach.toml
420 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/ledgeclimb_fd_peach.toml
420 ticks, 49 keys, 0 divergences
$ cargo run -q -p melee-sim -- gate harness/scenarios/ledgeescape_fd_peach.toml
420 ticks, 49 keys, 0 divergences
```

`crates/melee-sim/tests/m4_gate.rs` contains eighteen new scene tests plus eighteen
independent ordered particle ledger tests. The bone tests live in
`crates/melee-sim/src/frame/peach_bones.rs`:

```text
$ cargo test -p melee-sim --lib peach_bones -- --nocapture
idle_fd_peach: 114 Peach bones (45 dynamic), 73 Fox bones, 8 ticks, 13464 SRT words, 0 matrix words (no rendered capture)
start_fd_peach: 114 Peach bones (45 dynamic), 73 Fox bones, 130 ticks, 220168 SRT words, 0 matrix words (no rendered capture)
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out
```

Total: **233632 SRT words**. The comparison uses exact float bits and the existing
Marth contract: quaternion W is checked, unused Euler W is not. Tick-boundary
matrix caches are not rendered-pose oracles; there is no rendered Peach capture.

The remaining acceptance commands:

| Exact command | Result |
| --- | --- |
| `cargo test -p ft-peach` | 3 passed, 0 failed |
| `cargo test -p melee-sim --test m4_gate` | `test result: ok. 181 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 112.61s` |
| `cargo test -p hsd-particle` | 64 passed, 0 failed across 21 suite summaries |
| `cargo test -p melee-ft` | 89 passed, 0 failed across 23 suite summaries |
| `cargo gate` | Exit 0; 792 passed, 0 failed, 1 pre-existing ignored doctest across 129 suite summaries |
| `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; final line: ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 14.64s`` |
| `cargo fmt --all` | Exit 0; no output |

The multi-suite commands print one summary per executable; the numbers above
are sums of those summaries. Their trailing doctest summary is `test result: ok. 0 passed; 0 failed;
0 ignored; 0 measured; 0 filtered out; finished in 0.00s`.


## What changed

Character files:

- `crates/ft-peach/Cargo.toml`, `src/{lib,attributes,init}.rs`,
  `tests/attributes.rs`, `tests/support/mod.rs`.
- Workspace `Cargo.toml`, `Cargo.lock`, `crates/melee-sim/Cargo.toml` and
  `crates/melee-sim/src/scene_fighter.rs` register the crate and descriptor.

Shared implementation:

- `crates/melee-ft/src/fighter/{mod,spawn,procs,jump,fall}.rs`: resource-dependent
  load, grounded reset, force-start and float-input hooks; Peach's animation-driven
  aerial jump; default behavior retained for other character implementations.
- `crates/melee-ft/src/{dynamics/mod,fighter/assets}.rs`: decode each motion's
  dynamic-chain start indices using its blend metadata.
- `crates/melee-ft/src/fighter/{commands,dynamic_commands}.rs` and
  `crates/melee-ft/src/anim/playback.rs`: opcode 50, dynamic ownership transitions,
  subtree animation restoration at the current frame and retail sibling reset.
- `crates/melee-mp/src/map.rs` and `crates/melee-sim/src/frame.rs`: advance the
  collision stamp for static ground animation callbacks as retail does.
- `crates/melee-sim/src/initial_state/mod.rs`: validate/resume the inactive stock
  HUD boundary without replaying already completed simulation phases.

Verification and documentation:

- `crates/melee-sim/src/frame/peach_bones.rs`, the module declaration in `frame.rs`,
  and `crates/melee-sim/tests/m4_gate.rs`.
- `docs/PEACH_DATA.md`, this report, `fighter/README.md` and `TRACKER.md`.

## Assumptions exposed by Peach

1. A double jump need not use the ordinary initial vertical impulse or gravity.
   Peach's existing `AerialJumpStyle::Peach` hook now reaches her real entry and
   animation-displacement physics. Actual Float selection remains explicitly
   unsupported through her own callback.
2. OnLoad can depend on loaded animation lengths and costume identity. The new
   default `on_resources_loaded` hook supplies those resources without a kind
   branch in shared code. Grounded motion/landing reset her airborne resources.
3. Dynamics need a per-chain animation boundary, not just an enabled bit. Peach
   also changes those boundaries with subaction opcode 50. Restoring a joint's
   descriptor visits its following siblings even though attaching/evaluating
   animation is limited to its subtree. Start tick 100 exposes that distinction.
4. Static ground animation still advances the collision stamp. Omitting it made
   `ledgeescape_fd_peach` land at tick 250, one tick before retail. Updating the
   stamp selects the existing unextended floor-check path correctly.
5. An idle save can stop at link 17 inside stock HUD processing after particles
   and dynamics. The importer accepts only validated inactive effect conditions.

[PEACH_DATA.md](../../../../docs/PEACH_DATA.md) supplies archive layout, callback
addresses, float-operation audit references, data pins and the complete hook
inventory. Its one source discrepancy is the stale `types.h` item-array offset
comment (+1C instead of +18). The recorded scenarios and transition notes agree
with the passing gates; no test expectations were altered.

## Boundaries

Float entry, special moves, attacks and active item interactions are not ported.
Their input/interaction paths remain explicit unsupported branches; Peach's
supported double jump is fully implemented. The particle coverage here is the
complete scene RNG key and eighteen ordered ledgers, not a newly added full-field
Peach particle replay. SRT verification covers start 130 and idle eight ticks.

No Dolphin invocation, game-data changes, protected-directory edits or commits.
The worktree's pre-existing decomp type-change and untracked ROM/trace symlinks
are preserved. The lane is ready for review after the checks recorded above.
