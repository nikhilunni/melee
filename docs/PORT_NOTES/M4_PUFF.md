# Lane C: Jigglypuff idle, match start and movement

Implemented in `ft-purin`, with shared Kirby/Jigglypuff multijumps in
`melee-ft/fighter/multi_jump.rs`. No kind check was added to shared gameplay.
The work is uncommitted. Archive details are in [PUFF_DATA.md](../../../../docs/PUFF_DATA.md).

## Behavior and shared assumptions

| Previous assumption | Change |
| --- | --- |
| All supported aerial jumps use the common F/B pair | `AerialJumpStyle::MultiJump` and character hooks select attributes, action family and submotion; the shared callbacks handle F1..F5 |
| A jump always needs a fresh press/tap | The first multijump uses the ordinary edge predicate; later jumps accept held X/Y or up after the active jump script permits them |
| Every aerial-jump animation ends in FallAerial | Multijumps enter Fall while jumps remain; the final jump enters FallAerial |
| Backward aerial jumps retain facing | Multijumps rotate the model over the attribute duration and reverse facing halfway through; the threshold is strict `<` |
| All motion resources are common or idle-table entries | `CharacterDescriptor::additional_motions` loads the five character-table jump animations and scripts; existing descriptors supply an empty slice |
| Semantic motion identity is also its archive row | The common motion-entry implementation accepts a character row with explicit action ID and animation, retaining every shared entry side effect |
| Saved command stacks are empty | Import decodes counted-loop body/count pairs and subroutine continuations from saved `CommandInfo` and archive commands |
| Both fighters have completed setup at a start save | A narrowly checked costume-allocation boundary resumes the unfinished fighter from saved `StaticPlayer` through the existing cold constructor |
| New fighters start with disabled dynamics | Creation enables chains in the original costume pose before reset; allocation-only savestate import keeps its existing attach-then-restore sequence |

`multi_jump_attributes()` defaults to None; `multi_jump_family()` defaults to
ordinary family 0; an enabled character must implement `multi_jump_animation()`.
Puff overrides the attributes and animation hooks and selects the multijump style.
The second action family is data-driven and available for a future Kirby override.
All numeric jump differences come from ext_attr/common attributes.

The shared multijump keeps the vertical tilt timer (`ftCo_800CBAC4` argument
false), clears command variable 0 before motion entry, applies gravity on the
entry tick, and uses scaled drift without the ordinary additive drift base.
Collision follows `ft_80082F28` ground/ledge behavior without the StopCeil branch.

## Recording evidence and corrections

The supplied start sequence matches: 322 at tick 0, 323 at 6, 324 at 35,
29 at 65, 42 at 82, 14 at 112. Tick 35 Y is 11.272743225097656 (the notes'
rounded 11.3). `jump_fd_puff` enters 341 at 113 and lands directly at 155;
`airjumpb_fd_puff` enters 341 at 51, Fall at 101 and Landing at 115.

The start **savestate** is earlier than its tick-zero trace: Fox is still state
0 with zero facing/position. This is confirmed by `start_fd_puff.sav.json`.
The saved CPU is at **80363CD0**, inside `HSD_MObjAlloc`; the stack returns
through **80068F8C**, the costume-load call in `Fighter_Create`. The importer
reads the pending slot's saved `StaticPlayer` (base 80453080, stride E90),
including position, facing, costume, scale, damage, CPU settings and entry delay.
It does not copy the completed Fox from the expected trace.

The two remaining CPU draws are `ftCo_800A101C` and `ftCo_800B9704`, followed
by the existing alternate-music draw. The start ledger's seeds are
386572004 -> 1776767511 -> 713278590 -> 3460745545. This initialization interval
is now executed from the saved seed, without runtime ledger lookup.

Idle Puff's active command stack has depth 2: a counted loop, not two return
addresses. It restores from saved words and the actual preceding command.
The idle ledger requires no new RNG site. All eighteen ordered particle RNG
ledgers are tested independently of the scene's seed comparison.

`ftPurin/types.h` has stale types in the shared jump prefix. See the data report
for the `Fighter_x2D0_t`/assembly evidence; no header or fixture was edited.

## Arithmetic audit

Commands actually used from the lane root:

```sh
python3 harness/asm.py ftCo_800D74A4
python3 harness/asm.py ft_800CB6EC
python3 harness/asm.py ftCo_JumpAerialF1_Phys
python3 harness/asm.py ft_80084E1C
python3 harness/asm.py ftCo_800CBAC4
python3 harness/asm.py Fighter_Create --calls
python3 harness/asm.py Fighter_NewSpawn_80068E40
python3 harness/asm.py Player_GetDamage
python3 harness/asm.py Player_GetStocks
```

Launch X (800D74EC), backward comparison (800D7554), drift multipliers
(800D765C/64), and drift targets (80084EA8/AC) use separate `fmuls`.
Turning uses `fdivs` at 800CB768 and **fnmsubs at 800CB76C**, implemented with
`gekko_math::fma::fnmsubs` and the retail float PI/180 constant. Existing gravity,
clamping, dynamic initialization and solver arithmetic are reused unchanged.

## Bone and state coverage

| Capture | Ticks | SRT words | Result |
| --- | ---: | ---: | --- |
| `start_fd_puff.bones.jsonl` | 130 | 144091 | zero mismatches |
| `idle_fd_puff.bones.jsonl` | 8 | 8924 | zero mismatches |
| Total | 138 | 153015 | zero mismatches |

Both fighters are checked: all 50 Puff joints (three dynamic nodes) and all 73
Fox joints. The Marth contract is preserved: compare SRT at the tick boundary,
including quaternion W when used, excluding only unused Euler W. No rendered
matrix comparison is claimed. The start test also verifies freshly constructed
Fox tail dynamics, which the prior cold trace gates did not inspect.

Additional tests compare multijump turn countdown, retained scratch, command
variables and vertical tilt age against both recorded jump traces. A behavioral
Anim/IASA test holds X through all five aerial jumps, checks halfway facing
reversal, and verifies F1..F5 -> FallAerial and six total jumps.

## Validation

The eighteen CLI gates ran with the supplied local assets. Exact commands and final lines:

| Command | Final line |
| --- | --- |
| `cargo run -q -p melee-sim -- gate harness/scenarios/idle_fd_puff.toml` | `600 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/start_fd_puff.toml` | `600 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/squat_fd_puff.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/turn_fd_puff.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/walk_fd_puff.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/walkfast_fd_puff.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/dash_fd_puff.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/turnrun_fd_puff.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/jump_fd_puff.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/airjumpb_fd_puff.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/shield_fd_puff.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/spotdodge_fd_puff.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/roll_fd_puff.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/airdodge_fd_puff.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/wavedash_fd_puff.toml` | `300 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ledge_fd_puff.toml` | `420 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ledgeclimb_fd_puff.toml` | `420 ticks, 49 keys, 0 divergences` |
| `cargo run -q -p melee-sim -- gate harness/scenarios/ledgeescape_fd_puff.toml` | `420 ticks, 49 keys, 0 divergences` |

| Command | Result |
| --- | --- |
| `cargo test -p melee-sim --test m4_gate` | 256 passed, 0 failed, 0 ignored |
| `cargo test -p hsd-particle` | 70 passed, 0 failed, 1 ignored |
| `cargo test -p melee-ft` | 90 passed, 0 failed, 0 ignored |
| `cargo test -p melee-sim --lib puff_bones -- --nocapture` | 2 passed, 0 failed, 0 ignored |
| `cargo test -p melee-sim --lib puff -- --nocapture` | 4 passed, 0 failed; bones, recorded scratch and all-five-jumps behavior |
| `cargo gate` | 922 passed, 0 failed, 3 pre-existing ignores (144 suite summaries) |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0; `Finished dev profile [unoptimized + debuginfo] target(s) in 17.21s` |
| `cargo fmt --all` | exit 0, no output |
| `git -c core.fsmonitor=false diff --check` | exit 0, no output |

The new local oracles ran with assets present; none of the Puff checks skipped.
The three existing workspace ignores are the full grab gameplay test, its
particle replay, and the schema doctest. No expected values or tolerances changed.
`m4_gate.rs` adds eighteen scenario tests and eighteen ordered-ledger tests.

Logs: `/tmp/puff-final-gate.log`, `/tmp/puff-clippy.log`, and
`/tmp/puff-acceptance-{idle,start,squat,turn,walk,walkfast,dash,turnrun,jump,airjumpb,shield,spotdodge,roll,airdodge,wavedash,ledge,ledgeclimb,ledgeescape,m4,particle,ft,bones}.log`.
`/tmp/puff-acceptance-results.json` records each exact command, exit status and result.



## Limits

Colored hats, special moves, Kirby gameplay/helmet bodies, item interactions,
and unrecorded combat branches remain explicitly unsupported. No new full-field
Puff particle replay was added; scene RNG, ordered particle draw sites, and all
existing `hsd-particle` regressions are checked. No Dolphin invocation, protected
fixture/decomp modifications, commits or pushes were made. The lane's pre-existing
ROM/trace symlinks and decomp typechange remain intact.

## Files

- `Cargo.lock`
- `Cargo.toml`
- `TRACKER.md`
- `crates/ft-captain/src/init.rs`
- `crates/ft-falco/src/init.rs`
- `crates/ft-fox/src/init.rs`
- `crates/ft-mars/src/init.rs`
- `crates/ft-peach/src/init.rs`
- `crates/ft-purin/Cargo.toml`
- `crates/ft-purin/src/attributes.rs`
- `crates/ft-purin/src/init.rs`
- `crates/ft-purin/src/lib.rs`
- `crates/ft-purin/tests/attributes.rs`
- `crates/ft-purin/tests/support/mod.rs`
- `crates/ft-yoshi/src/init.rs`
- `crates/melee-ft/src/fighter/M4_PUFF.md`
- `crates/melee-ft/src/fighter/README.md`
- `crates/melee-ft/src/fighter/assets.rs`
- `crates/melee-ft/src/fighter/fall.rs`
- `crates/melee-ft/src/fighter/jump.rs`
- `crates/melee-ft/src/fighter/landing.rs`
- `crates/melee-ft/src/fighter/mod.rs`
- `crates/melee-ft/src/fighter/multi_jump.rs`
- `crates/melee-ft/src/fighter/procs.rs`
- `crates/melee-ft/src/fighter/spawn.rs`
- `crates/melee-ft/src/fighter/state.rs`
- `crates/melee-ft/src/physics/airborne.rs`
- `crates/melee-sim/Cargo.toml`
- `crates/melee-sim/src/frame.rs`
- `crates/melee-sim/src/frame/puff_bones.rs`
- `crates/melee-sim/src/frame/puff_state.rs`
- `crates/melee-sim/src/initial_state/fighter.rs`
- `crates/melee-sim/src/initial_state/mod.rs`
- `crates/melee-sim/src/initial_state/setup_resume.rs`
- `crates/melee-sim/src/scene_fighter.rs`
- `crates/melee-sim/tests/m4_gate.rs`
- `docs/PUFF_DATA.md`
