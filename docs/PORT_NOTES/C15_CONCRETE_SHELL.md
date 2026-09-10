# C15: concrete fighter shell and static character callbacks

Implemented and verified, 2026-09-10, combat lane, base `82459f1`. Uncommitted
per task; no Git write commands were run.
The pre-existing decomp symlink is unchanged; no protected harness path was edited.

## Before measurement on the merged S1/S4 tree

The initial strict `cargo gate` passed 1,013 tests, failed 0, with the same three
existing ignores. Fresh performance evidence: `target/perf/20260910T074240Z-77868`.
The PERF substring census includes every function name containing `melee_ft::`,
including trait adapters and standard-library helpers mentioning those types.
The character-family crate and the unimplemented Mario crate are measured too.

| Compiling crate | Before copies | Before IR lines | After copies | After IR lines |
|---|---:|---:|---:|---:|
| ft-captain | 2 | 148 | 66 | 434 |
| ft-falco | 22 | 890 | 67 | 434 |
| ft-fox | 22 | 890 | 67 | 434 |
| ft-fox-family | 0 | 0 | 0 | 0 |
| ft-mario | 0 | 0 | 0 | 0 |
| ft-mars | 3 | 131 | 66 | 402 |
| ft-peach | 8 | 408 | 67 | 678 |
| ft-purin | 2 | 74 | 66 | 351 |
| ft-yoshi | 22 | 1,105 | 68 | 996 |
| melee-ft | 561 | 82,190 | 845 | 103,736 |
| melee-sim | 2,112 | 144,591 | 128 | 3,197 |

Before: 2,754 total charged definitions; 3,916,784 stripped bytes; 3,489,792
text bytes; 166.264 ms load; 23.867 ms per 600 ticks (about 25,140 ticks/s).
This reproduces the existing S1/S4 copy regression against P1 without changing
or promoting that baseline. P1 remains the time and size acceptance reference.

The clean build uses a new empty target directory, identical Cargo home/offline
settings and compiler options, and `cargo build --release -p melee-sim`.
Before target `/private/tmp/c15-clean-before`: 10.413 seconds wall, exit 0.
This includes workspace and dependency compilation; it does not flush the OS
filesystem cache. The before log and timing JSON live in `target/c15-evidence/`.

## Ownership and static tables

`Fighter` is concrete: `FighterCore`, inline `CharacterState`, and the installed
`MotionRow`. There is one `state::COMMON` static containing all 341 common rows.
Each character crate defines a `static TABLE: CharacterTable`, whose const
constructor binds its hooks and concrete special rows. Common methods and the
four pair helpers take concrete fighters; no character generic reaches them.
Pair operations continue to invoke each participant's own character table.

This implementation uses the task's permitted aligned inline-store alternative
to the design note's enum. Existing attribute structs, archive readers and typed
move scratch remain in their current character/family crates. No reverse crate
dependency or payload allocation is introduced. The private 1,024-byte storage
has 16-byte alignment; construction checks capacity, alignment and table type
identity. `get::<C>()` and `get_mut::<C>()` check the stored TypeId before borrowing
the initialized value. The stored destructor runs exactly once, including after
moves; existing resource vectors retain their ownership and initialization cost.
The small unsafe implementation is confined to initialization, checked borrows
and destruction; callers cannot inspect or replace the raw bytes or type tag.
`CharacterCallbacks: Send + Sync` preserves those auto-trait guarantees for the
inline owner. New ownership tests exercise moves, aligned data, destruction,
wrong typed accesses, mismatched tables and rejected capacity/alignment.

Static table construction preserves each existing hook's signature and defaults.
Typed receiver hooks are adapted to `&CharacterState` / `&mut CharacterState`;
whole-fighter hooks take concrete `&mut Fighter`. Borrowed return values such as
multi-jump attributes retain the payload lifetime. No hook calls are combined.
The installed row remains authoritative for each scheduler phase, including when
an earlier callback replaces it. Motion changes retain begin-change, grounded
hook, reset/install, animated-shield hook and playback in their original order.
Revival retains P1's owners and resets through the same table's death hook.

## Measured layout on arm64

A small external Rust probe linked the actual before and after release rlibs
(without adding a workspace target). Raw output: `target/c15-evidence/*-layout.txt`.

| Owner | Before bytes / alignment | After bytes / alignment |
|---|---:|---:|
| FighterCore | 38,696 / 8 | 38,696 / 8 |
| MotionRow | 56 / 8 | 56 / 8 |
| Fighter | 38,928..39,256 / 8, depending on character | 39,808 / 16 |
| CharacterState inline owner | typed payload plus separate table references | 1,056 / 16 |
| CharacterTable | absent | 336 / 8 |
| Common row array | seven character-typed arrays of 19,096 bytes | one static array of 19,096 bytes |

The typed payloads are unchanged: Fox/Falco 248 bytes, Marth 164, Falcon 148,
Peach 264, Yoshi 480 and Jigglypuff 236. The 1,024-byte store deliberately has
headroom beyond the largest current payload; its cost is 552..880 additional
bytes per initialized fighter, with no added allocation count. Seven character
tables occupy 2,352 bytes before any linker folding. Removing six common arrays
saves 114,576 bytes of logical row data (81,840 bytes of callback pointers),
separate from the measured linked segment sizes reported below.

## Migration for S2/S3/S5 through S10

- Replace `Fighter<C>` / `Fighter<Self>` with `Fighter`; `MotionRow<C>` and the
  five phase fn aliases likewise lose their type parameter. Existing phase
  resources and return types are unchanged.
- Common callbacks lose their `<C: CharacterCallbacks>` parameter and references
  lose `::<C>`. Their definitions and common rows stay in melee-ft.
- Motion entry takes `ActionId`: call `change_motion_state(state.into(), assets)`.
  Rate, source, frame and other argument ordering are unchanged.
- Implement `CharacterCallbacks` on the character's existing typed state. Define
  `static TABLE: CharacterTable = CharacterTable::new::<Character>()` and return
  `&TABLE` from `table()`. Define special rows through `SPECIAL_ROWS`, a static
  slice of concrete `MotionRow`; the former `COMMON` associated const is gone.
  Reuse rows from `melee_ft::fighter::state::COMMON` when appropriate.
- Whole-fighter hooks access scratch through `fighter.character.get::<Character>()`
  or `get_mut::<Character>()`. Release that borrow before calling a motion
  transition or another whole-fighter hook. Ordinary typed `&self`/`&mut self`
  implementations retain their signatures; the const table builder adapts them.
- Construction passes `character.into_state()` to `Fighter::prepare`, `spawn` or
  `spawn_for_match`. Those common construction bodies are also concrete.
- FoxFamily retains `rows::<C>()` and member-dependent callbacks, but their fighter
  argument is concrete. Call member-dependent helpers with explicit `::<C>`;
  C is no longer inferred from the fighter. Common physics/collision/camera
  pointers in family rows have no type argument.
- `scene_characters!` remains the single roster list for archive selection and
  construction. `SceneFighter` owns one `Box<Fighter>` allocated at setup.
  `with_fighter!` now unwraps that box with one pattern, with no per-kind match.

## What stays generic and why

No `Fighter<C>`, generic `MotionRow`, common phase callback or pair helper remains
in Rust source. Motion-entry conversion wrappers also take concrete `ActionId`,
so calling with common and family state enums cannot instantiate multiple shell
bodies. The table authoring trait and typed payload access/construction/drop
adapters remain parameterized by the character type. They perform checked typed
access or select hooks; they never instantiate the common state graph.
`scene_fighter::construct<C>` remains an initialization-only archive adapter.
Existing lower-level math/iterator/closure helpers are outside the shell and
remain subject to the complete substring census.

The family census uses the same LLVM output, selecting `ft_fox_family::` anywhere
in a label. It is separate from the mandatory `melee_ft::` substring accounting.

| Compiling crate | Before family copies / IR lines | After family copies / IR lines |
|---|---:|---:|
| ft-fox | 16 / 1,103 | 16 / 1,145 |
| ft-falco | 16 / 1,103 | 16 / 1,145 |
| ft-fox-family | 2 / 77 | 2 / 77 |
| Every other measured crate | 0 / 0 | 0 / 0 |
| Total | 34 / 2,283 | 34 / 2,367 |

Per member, `start`, `firing` and `end` each have two AIR instantiations;
`enter_special`, `fire`, `item_owner`, `control`, `update_blaster`, `loop_input`
and `accessory` each have one. These 13 move definitions per member use family
constants or typed scratch access; there are also two member accessor definitions.
The final per-member definition is the non-generic private empty `no_input`,
materialized in each crate evaluating the const family rows. `hold_position` and
`item_muzzle` are the two concrete definitions owned by ft-fox-family itself.
The const `rows::<C>()` builder emits no runtime definition. Its scalar `row`
builder and `no_input` no longer have generic parameters. Common callbacks in
these rows refer directly to the one melee-ft implementation.

A broad common-definition audit additionally found the existing revival map
helper's two equivalent closure expressions grouped as two definitions under
one LLVM label. It now binds the pose lookup closure once before the unchanged
Rebirth/RebirthWait branch. Both full workspace profiles were rerun after this
change. No float expression, callback order or collision predicate changed.

The full census deliberately still includes compiler-generated adapters/drop
helpers and all their copies. A low sim count cannot conceal movement of the
common graph into character crates: the gate checks aggregate counts and the
common labels across compiling crates, preserving the raw attribution JSON.

## Final census and performance

Final evidence: `target/perf/20260910T080958Z-957`, `target/c15-evidence/final-census.json` and
`combined-common-definitions.json`. The unchanged substring census totals
**2,754 → 1,440 definitions (-47.71%)**, **230,427 → 110,662 IR lines (-51.98%)**.
melee-sim drops **2,112 → 128** copies; melee-ft owns **845**, below 900 as well.
All **270** audited common labels, including helper closures and the concrete
Snapshot implementation, have one definition across compiling crates. Each of
`capture_pair`, `enter_back_throw`, `release_back_throw` and `detect_hit` falls
from 49 definitions to one. Character crates contain adapters, not the common
graph. Raw labels are retained without renaming or character-count division.

The additional `cargo llvm-lines -p melee-sim --release --bin melee-sim`
check contributes 45 substring-matching compiler/library helper definitions and
**zero common labels**. The main before/after table uses the same required
library scope in both measurements; it does not silently mix binary and library
counts. Including the binary leaves all 270 common labels defined exactly once.

| Metric | Before merged S1/S4 | Final C15 |
|---|---:|---:|
| Stripped bytes | 3,916,784 | 3,483,104 |
| Text segment | 3,489,792 | 3,178,496 |
| `__DATA_CONST,__const` | 246,440 | 133,728 |
| Load mean | 166.264 ms | 165.646 ms |
| 600 simulate-only ticks | 23.867 ms | 23.533 ms |
| Throughput | 25,140 ticks/s | 25,496 ticks/s |
| Clean release build wall time | 10.413 s | 9.711 s |

The final binary is **264,528 bytes below P1's 3,747,632-byte ceiling**, paying
both S1 and S4's size debt. Throughput stays within the P1 tolerance; the samples
do not establish a statistically significant throughput improvement. The build
wall times are individual measurements, not a statistical build-speed claim.
The final empty target was `/private/tmp/c15-clean-final`, checked absent before
building, with identical offline Cargo-home settings and no rustc wrapper.
`before-sections.txt`, `after-sections.txt` and `after-symbols.txt` retain the
Mach-O evidence: one linked COMMON symbol and seven static character TABLEs.

The final performance guard ratchets core/sim to **845/128**, character crates
to their exact measured counts, and all measured libraries to **1,440** total.
It also enforces one common definition across crates, the fixed P1 stripped
ceiling, and P1 time ceilings. Ordinary +10% time / +5% size / +0 copy comparison
against the last complete PASS remains active and cannot raise those fixed caps.
The first post-refactor sample failed only the old per-crate attribution;
its record remains unchanged. The explicit architecture migration and both
later COMPLETE PASS blocks are in PERF. Nine performance-tool unit tests pass,
including cross-crate duplicate detection, missing pairs and fixed ceiling drift.
The existing synthetic successful fixture now contains all four required pair
helper definitions; no retail expected word or behavior assertion was changed.

## Final acceptance and exact lines

All commands exited zero. Both final workspace runs pass **1,017**, fail **0**,
and retain the same **3** pre-existing ignores. M4 is **261**, M5 is **28**, with
S1's nineteen gates and S4's laser gate included in both profiles. hsd-particle
passes **80** tests; all nine character/family crate suites pass **19** tests
combined (Mario remains a placeholder). The four additional workspace tests
are the inline-owner regressions; no oracle expectation or ignore was changed.

The full command/exit manifests are `validation.json` and
`final-validation.json` in `target/c15-evidence/`. The final two workspace runs
follow the revival closure deduplication. The earlier standalone M4/M5,
particle, character, allocation and math runs also pass; those suites run again
inside the final full workspace profiles where applicable. Exact final lines:

`cargo gate`:

```text
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo gate --release`:

```text
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo test -p melee-sim --test m4_gate`:

```text
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 86.45s
```

`cargo test -p melee-sim --test m5_gate`:

```text
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 17.55s
```

`cargo test -p hsd-particle`:

```text
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo test -p ft-fox -p ft-falco -p ft-mars -p ft-captain -p ft-peach -p ft-yoshi -p ft-purin -p ft-mario -p ft-fox-family`:

```text
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo test -p melee-sim --test alloc_gate -- --nocapture`:

```text
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.08s
```

`tools/check-release-math.sh`:

```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo clippy --workspace --all-targets -- -D warnings`:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 33.33s
```

`cargo fmt --all -- --check`:

```text
(no output; exit 0)
```

The workspace final lines are its final empty doctest suite; the aggregate counts
above sum every test-result line. `check-release-math.sh` passes 74 tests including
opt levels 0, 1, 2, 3, s and z. Installed-row callback substitution, character
state mapping, mixed-character combat and repeated-revival owner retention all
remain covered by the final workspace runs.

Allocation ceilings remain 17 / 1 / 12 / 22 / 17 for start FD, idle FD, Marth jab,
Marth KO and start BF; the S1 zero ceilings and S4 laser ceiling 32 are unchanged.
Measured simulate-only allocations are 0 in those non-KO legacy scenes, both
S1 scenes and laser, and 1 in KO. Repeated revival remains zero-allocation.
Snapshot accounting remains separate. Exact measured allocation lines:

```text
laser_fd_fox: 299 measured ticks; simulate-only 0 (0.000000/tick), peak 0, allocating ticks 0; with snapshot 37375 (125.000000/tick), peak 125; snapshot overhead 37375 (125.000000/tick)
fsmashcharge_fd_fox: 299 measured ticks; simulate-only 0 (0.000000/tick), peak 0, allocating ticks 0; with snapshot 37375 (125.000000/tick), peak 125; snapshot overhead 37375 (125.000000/tick)
jabcombo_fd_fox: 299 measured ticks; simulate-only 0 (0.000000/tick), peak 0, allocating ticks 0; with snapshot 37375 (125.000000/tick), peak 125; snapshot overhead 37375 (125.000000/tick)
start_bf_fox: 599 measured ticks; simulate-only 0 (0.000000/tick), peak 0, allocating ticks 0; with snapshot 74875 (125.000000/tick), peak 125; snapshot overhead 74875 (125.000000/tick)
start_fd_fox: 599 measured ticks; simulate-only 0 (0.000000/tick), peak 0, allocating ticks 0; with snapshot 74875 (125.000000/tick), peak 125; snapshot overhead 74875 (125.000000/tick)
idle_fd_fox: 599 measured ticks; simulate-only 0 (0.000000/tick), peak 0, allocating ticks 0; with snapshot 74875 (125.000000/tick), peak 125; snapshot overhead 74875 (125.000000/tick)
jab_fd_marth: 299 measured ticks; simulate-only 0 (0.000000/tick), peak 0, allocating ticks 0; with snapshot 37375 (125.000000/tick), peak 125; snapshot overhead 37375 (125.000000/tick)
ko_fd_marth: 479 measured ticks; simulate-only 1 (0.002088/tick), peak 1, allocating ticks 1; with snapshot 59876 (125.002088/tick), peak 126; snapshot overhead 59875 (125.000000/tick)
```

`CARGO_HOME=/private/tmp/p1-cargo-home CARGO_NET_OFFLINE=true tools/perf-gate.sh`
(default tolerances, ratcheted fixed thresholds, exit 0):

```text
[PASS] perf-gate: 3483104 stripped bytes, 3178496 text bytes; load 165.646 ms; ticks_600 23.533 ms
```

## Files and limits

No acceptance command is blocked. Existing three ignored tests remain explicit
pre-existing scope limits; this refactor does not implement their missing behavior.
No game data, traces, scenarios, decomp contents, expected retail values or
allocation ceilings were edited. No compiler profile, dependency or benchmark
parameter changed. The decomp type-change in Git status predates this task.

Changed files (the existing decomp symlink is excluded):

- `CLAUDE.md`
- `TRACKER.md`
- `crates/ft-captain/src/init.rs`
- `crates/ft-falco/src/init.rs`
- `crates/ft-fox-family/src/special_n.rs`
- `crates/ft-fox/src/init.rs`
- `crates/ft-mars/src/init.rs`
- `crates/ft-peach/src/init.rs`
- `crates/ft-purin/src/init.rs`
- `crates/ft-yoshi/src/init.rs`
- `crates/ft-yoshi/src/shield.rs`
- `crates/melee-ft/src/fighter/air_dodge.rs`
- `crates/melee-ft/src/fighter/attack.rs`
- `crates/melee-ft/src/fighter/character.rs`
- `crates/melee-ft/src/fighter/character/tests.rs`
- `crates/melee-ft/src/fighter/damage.rs`
- `crates/melee-ft/src/fighter/dash.rs`
- `crates/melee-ft/src/fighter/down.rs`
- `crates/melee-ft/src/fighter/entry.rs`
- `crates/melee-ft/src/fighter/escape.rs`
- `crates/melee-ft/src/fighter/fall.rs`
- `crates/melee-ft/src/fighter/grab.rs`
- `crates/melee-ft/src/fighter/grab_throw.rs`
- `crates/melee-ft/src/fighter/jump.rs`
- `crates/melee-ft/src/fighter/landing.rs`
- `crates/melee-ft/src/fighter/ledge.rs`
- `crates/melee-ft/src/fighter/life.rs`
- `crates/melee-ft/src/fighter/mod.rs`
- `crates/melee-ft/src/fighter/multi_jump.rs`
- `crates/melee-ft/src/fighter/pass.rs`
- `crates/melee-ft/src/fighter/procs.rs`
- `crates/melee-ft/src/fighter/run.rs`
- `crates/melee-ft/src/fighter/shield.rs`
- `crates/melee-ft/src/fighter/smash.rs`
- `crates/melee-ft/src/fighter/snapshot.rs`
- `crates/melee-ft/src/fighter/spawn.rs`
- `crates/melee-ft/src/fighter/squat.rs`
- `crates/melee-ft/src/fighter/state.rs`
- `crates/melee-ft/src/fighter/state/callbacks/animation.rs`
- `crates/melee-ft/src/fighter/state/callbacks/camera.rs`
- `crates/melee-ft/src/fighter/state/callbacks/collision.rs`
- `crates/melee-ft/src/fighter/state/callbacks/input.rs`
- `crates/melee-ft/src/fighter/state/callbacks/physics.rs`
- `crates/melee-ft/src/fighter/state/common_table.rs`
- `crates/melee-ft/src/fighter/state/phase.rs`
- `crates/melee-ft/src/fighter/state/row.rs`
- `crates/melee-ft/src/fighter/state/special.rs`
- `crates/melee-ft/src/fighter/turn.rs`
- `crates/melee-ft/src/fighter/turn_run.rs`
- `crates/melee-ft/src/fighter/walk.rs`
- `crates/melee-ft/tests/fighter_graphics.rs`
- `crates/melee-ft/tests/fighter_support/mod.rs`
- `crates/melee-ft/tests/fighter_support/replay.rs`
- `crates/melee-ft/tests/fox_spawn_native.rs`
- `crates/melee-sim/src/frame.rs`
- `crates/melee-sim/src/frame/combat.rs`
- `crates/melee-sim/src/frame/falco_bones.rs`
- `crates/melee-sim/src/frame/falcon_bones.rs`
- `crates/melee-sim/src/frame/fall_states.rs`
- `crates/melee-sim/src/frame/marth_bones.rs`
- `crates/melee-sim/src/frame/peach_bones.rs`
- `crates/melee-sim/src/frame/puff_bones.rs`
- `crates/melee-sim/src/frame/puff_state.rs`
- `crates/melee-sim/src/frame/yoshi_bones.rs`
- `crates/melee-sim/src/frame/yoshi_state.rs`
- `crates/melee-sim/src/initial_state/fighter.rs`
- `crates/melee-sim/src/scene_fighter.rs`
- `crates/melee-sim/tests/alloc_gate/revival.rs`
- `docs/PERF.md`
- `docs/PORT_NOTES/C15_CONCRETE_SHELL.md`
- `tools/perf-gate.sh`
- `tools/perf_report.py`
- `tools/tests/test_perf_gate.py`
