# C8: shared subactions and hit collision

2026-09-09, combat lane. Uncommitted; no Git write commands were run. The
pre-existing decomp symlink is unchanged. No game assets, recordings, scenarios,
or expected oracle values were edited. Validation results are recorded below.

## Ownership and dependency direction

`melee-cmd` depends only on `melee-types` and `gekko-math`. `melee-coll` depends
only on those two crates and `hsd-types`. Neither depends on fighters, items,
stages, archives, effects, or the simulator. Consumers import the owning crate
directly; no cross-layer re-export facades remain at the old paths.

| Location | Responsibility |
|---|---|
| `melee-cmd/src/decode.rs` | Supported opcode decoding, word counts, graphics/hit/throw payload decoding. Archive relocation and reachable-script discovery stay in fighter assets. Graphics payload decoding is also used by the differently numbered color-overlay opcode. |
| `melee-cmd/src/state.rs` | Frame sampling, synchronous/asynchronous timers, animation-wrap waits, counted loops, calls/returns, jumps and termination. `next` returns one borrowed typed command, so its consumer finishes application before interpretation continues. |
| `melee-cmd/src/lib.rs` | Command vocabulary, color requests and charge scaling. No Fighter or character-type parameter. |
| `melee-types/src/combat.rs` | Shared hit/throw/graphics payloads and vulnerability enum. Pure data shared by cmd and coll without a dependency between them. |
| `melee-types/src/fixed.rs` | Inline storage moved from melee-ef and extended for control stacks/history; effects use the same implementation. |
| `melee-coll/src/hitbox.rs`, `hurtbox.rs` | HitCapsule phases, group history and spawn replacement; HurtCapsule data and hurt-table height. Bone ownership stays with consumers. |
| `melee-coll/src/detection.rs` | Hit-ID pair cursor, eligibility filtering, first hurt-table contact, and group victim recording. Each candidate reborrows current history after the prior hit is applied. |
| `melee-coll/src/geometry.rs` | Audited lbcollision capsule contact moved from melee-lb, including its existing bit-exact geometry tests. |
| `melee-coll/src/damage.rs` | Received-hit data, knockback arithmetic, hitlag calculation and expiry. Fighter launch states, DI boundaries, shield reactions and effect requests stay in melee-ft. |
| `melee-coll/src/defense.rs` | Reflect/absorb descriptors and integer clank-priority comparisons. |
| `melee-coll/src/overlap.rs` | Body separation arithmetic; the caller supplies stage adjacency lazily. |
| `gekko-math/src/matrix.rs` | Existing audited inverse/point-transform kernels over arrays; HSD and collision share them without pulling hsd-anim into melee-coll. HSD entry points retain their signatures. |
| `melee-ft/src/fighter/commands.rs`, `dynamic_commands.rs` | Apply commands to fighter animation, effects, charge, visibility, hitboxes, variables and dynamics. Dynamic ownership remains fighter-specific. |
| `melee-ft/src/fighter/{hitbox,caches,overlap,damage}.rs` | Bone sampling, fighter eligibility, character-hook boundary, reactions and stage context around the shared algorithms. |

`melee-ft/src/collision/{air,ground,ecb,pose}.rs` is entirely fighter-versus-stage
collision and stays in melee-ft. `attack.rs` is jab/tilt entry, input priority
and grounded physics; it contains no independent hit-detection machinery to move.

Retail sources inspected: `lb/lbcommand.c`, `ft/ftaction.c`, `ft/ftcoll.c`,
`lb/lbcollision.c`, `lb/types.h`, and item dispatch in `it/itanimlist.c`
(`it_802799E4`, which calls `Command_Execute`). The pinned source uses
`itanimlist.c` rather than an `itcmd.c` filename for that dispatcher. Existing
float expressions, widths, FMA calls, capsule ordering and exception boundaries
are preserved. The new clank-priority helper performs integer comparisons after
the same `fctiwz` damage conversion (ftColl_8007699C).

## Collider contract and remaining combat breadth

`Collider` exposes hurt count, grabbability, lazy sampling of one hurt capsule
and its matrix, and owner scale. `FighterCore` implements lazy JObj sampling;
`PositionedCollider` implements already-positioned capsule/matrix tables for an
item owner. A unit test exercises the latter through the same contact loop,
including Catch skipping non-grabbable capsules. The loop stops at the first
contact, preserving the original matrix-cache demand order.

The scene still supplies receiver-first entity order. `PairCursor` visits hit
IDs in that pair, filtering ground/air eligibility, Catch and existing victims.
The fighter wrapper calls its character hook at the original per-candidate
boundary; shield contact still precedes hurt contact. Generic character wrappers
remain thin; the shared state/timer/geometry bodies are concrete.

This is an extraction of the supported combat slice. Fighter clank rebound
states, simultaneous damage selection, powershield response and item spawning
were explicit port gaps before C8 and remain so. The existing clank guard moved
to melee-coll; the independently tested priority result and reflect/absorb
payloads do not pretend to implement those owner reactions. No placeholder item
engine or new ungated fighter reaction was installed.

## Scheduler ownership

Both production `Rc<RefCell<...>>` objects in frame.rs are removed. HSD procs
can now hold a registration token rather than a closure capturing runtime state.
`World::run_procs_with` borrows a concrete dispatcher for one scheduler pass and
preserves the same traversal, pause, frame-tag, insertion and deferred-removal
logic. Simulation owns its runtime uniquely in a `Box` allocated during
construction; ticking neither allocates that box nor performs reference-count
or dynamic-borrow operations. The production registrations contain no boxed
callbacks; legacy engine callback support remains for existing engine users.

An inline Runtime first caused `start_dl_fox_cold_600` to overflow the normal
Rust test-thread stack. Unique initialization-time storage fixed it; the same
test passes without changing stack limits. The new scheduler regression test
checks in-frame proc creation and deferred self-destruction with a borrowed
owner. Remaining Rc/RefCell occurrences in frame.rs are under cfg(test), in the
unchanged legacy-callback tests.

## Allocation changes and explicit bounds

Part resources prepare their immutable AObj/FObj templates at asset load.
`add_prepared_joint_anim` reuses each joint's already-reserved track vector,
clones only immutable stream handles, sorts/reset-attaches the same tracks, and
preserves the original flags and frame-zero evaluation. Part bone selections
use the existing 140-part bound. Command side queues, texture selections, model
selections and group victim history use inline storage.

| Storage | Bound and overflow policy |
|---|---|
| Interpreter loops / return stack | 16 each, explicit port bound. The decomp labels `event_return[3]` a guessed size; this is not presented as a retail constant. Overflow fails before writing. |
| Fighter side request queues | 64 each, explicit port bound consistent with the existing effect request queue; consumers must drain requests before exhaustion. No request is silently dropped. |
| Model selections | 128 entries, signed seven-bit group field from opcode 31. |
| Texture selections | 128 unique indices, seven-bit field from opcode 40. |
| Part joints | 140, existing MAX_FT_PARTS bound. |
| Group victim history | 24 inline identities, explicit port bound matching the combined size of retail's two 12-entry arrays; this does not claim the unported two-list lifetime rules are implemented. |

The interpreter allocation test steps loops, calls and fractional timers with
allocation counting enabled and observes zero allocations. The scene census
retains its original one-tick warm-up and measures every remaining tick,
including first-use particle storage and respawn reconstruction. Only the five
ceilings were tightened; snapshot ownership and all oracle assertions are unchanged.

| Scene | Measured ticks | Before | After / ceiling | Reduction | Peak allocations in one tick |
|---|---:|---:|---:|---:|---:|
| start_fd_fox | 599 | 1,694 | 17 | 98.996% | 5 |
| idle_fd_fox | 599 | 2,116 | 1 | 99.953% | 1 |
| jab_fd_marth | 299 | 639 | 12 | 98.122% | 2 |
| ko_fd_marth | 479 | 1,517 | 690 | 54.515% | 668 |
| start_bf_fox | 599 | 1,431 | 17 | 98.812% | 6 |

Zero allocations for whole scenes is not claimed. Sampled backtraces identify
particle Vec growth, generator/AppSRT creation and draw-log storage. The KO
scene also reconstructs the fighter on revival: archive/JObj import and
`FighterCore::prepare` track provisioning account for its 668-allocation spike.
These are outside subaction stepping and capsule detection. Temporary allocation
probe code was removed; final totals come from the unchanged counting method in
`alloc_gate`, not the diagnostic probe. Snapshot overhead remains exactly 125
allocations per measured tick in all five scenes.

## LLVM instantiation census

The exact unqualified `cargo llvm-lines -p melee-sim --release` exits 101
because the package has both library and binary targets. Measurements use
`--lib` for melee-sim, melee-ft, melee-cmd and melee-coll, and also inspect the
simulator with `--bin melee-sim`. Profiles and compiler flags are unchanged.
The pre-change library samples were taken from a temporary HEAD source copy.

Counts below include namespace-owned functions and implementations, including
named closure groups and derived methods, and exclude std helpers merely
parameterized by those types. Totals sum emitted definitions across targets;
they are not a linked-machine-code size measurement.

| Compiling library | melee-ft lines before | After | melee-ft copies before | After |
|---|---:|---:|---:|---:|
| melee-sim | 138,056 | 136,292 | 1,885 | 1,885 |
| melee-ft | 79,003 | 76,136 | 421 | 415 |
| Total | 217,059 | 212,428 | 2,306 | 2,300 |

The performance script uses a broader substring census, also charging std
helpers parameterized by fighter types. Under that unchanged counting rule,
melee-ft's compiling library falls from **583 to 532 copies** and melee-sim
from **2,043 to 2,017**. Its separately measured character crates also pass their
recorded budgets. These counts differ from the namespace-only table by design.

The simulator binary target emits no namespace-owned melee-ft/cmd/coll bodies.
All **7 melee-cmd labels / 7 copies** and **15 melee-coll labels / 15 copies**
have exactly one emitted copy across the measured library targets. Small
fully inlined or unused routines have no standalone LLVM row. The shared
initialization-only graphics payload decoder is kept out of downstream IR with
`inline(never)` so the overlay loader and subaction decoder share one body.
The group-history search is a named helper, avoiding ambiguous closure display
names; no character-product instantiation was introduced.

| Compiling crate | Copies | New-crate function label |
|---|---:|---|
| melee-cmd | 1 | `melee_cmd::decode::decode` |
| melee-cmd | 1 | `melee_cmd::state::ScriptState::next` |
| melee-cmd | 1 | `melee_cmd::decode::hitbox` |
| melee-cmd | 1 | `melee_cmd::decode::throw_hitbox` |
| melee-cmd | 1 | `melee_cmd::decode::graphics` |
| melee-cmd | 1 | `melee_cmd::state::ScriptState::restart` |
| melee-coll | 1 | `melee_coll::geometry::capsule_contact` |
| melee-coll | 1 | `melee_coll::geometry::endpoint_projection` |
| melee-coll | 1 | `melee_coll::hitbox::group_history` |
| melee-coll | 1 | `melee_coll::detection::record_victim` |
| melee-coll | 1 | `melee_coll::detection::PairCursor::next` |
| melee-coll | 1 | `melee_coll::hitbox::spawn` |
| melee-coll | 1 | `melee_coll::hurtbox::HurtHeight::from_retail` |
| melee-coll | 1 | `melee_coll::detection::require_uncontested_hit` |
| melee-coll | 1 | `melee_coll::detection::record_victim::{{closure}}` |
| melee-ft | 1 | `melee_coll::detection::first_contact` |
| melee-ft | 1 | `<melee_cmd::Command as core::clone::Clone>::clone` |
| melee-ft | 1 | `melee_coll::damage::KnockbackParameters::knockback` |
| melee-ft | 1 | `<melee_coll::hurtbox::HurtHeight as core::fmt::Debug>::fmt` |
| melee-ft | 1 | `<melee_coll::hitbox::HitCapsule as core::clone::Clone>::clone` |
| melee-ft | 1 | `<melee_coll::hurtbox::HurtCapsule as core::clone::Clone>::clone` |
| melee-sim | 1 | `melee_coll::overlap::nudge` |

## Binary size follow-up

The first performance run failed its unchanged size ceilings: 4,029,424 stripped
bytes / 3,588,096 text bytes. A clean pre-change HEAD build measured 3,830,616 /
3,391,488, so this was a C8 regression. Function-level cargo-bloat localized
roughly 170 KiB of growth to the seven character importers: each repeated
construction of the new fixed part-joint arrays. `restore_part_animations` now
takes `FighterCore` and restores that shared state in one concrete helper,
retaining its original call position. This reduced the release binary to
3,864,176 stripped bytes / 3,424,256 text bytes, below both unchanged limits.
The failed performance block remains in docs/PERF.md; it was not promoted to a
baseline or removed. Final performance-gate results follow below.

## Validation

Commands ran with the local oracle data present, no missing-data opt-out and no
stack-limit or float/compiler-flag overrides. The completed workspace gates each
report **977 passed, 0 failed, 3 ignored** when their individual test-result lines
are summed. The three existing ignores are the two throw/capture scratch probes
in frame::combat and the Snapshot schema documentation example. No ignores or
expected oracle values were changed.

| Command | Result |
|---|---|
| `cargo build --workspace --all-targets --locked` | PASS |
| `cargo gate` | 977 passed / 0 failed / 3 existing ignores |
| `cargo gate --release` | 977 passed / 0 failed / 3 existing ignores |
| `cargo test -p melee-sim --test m4_gate`, debug and release | 261 passed in each profile |
| `cargo test -p melee-sim --test m5_gate`, debug and release | 8 passed in each profile |
| `cargo test -p hsd-particle` | 75 passed |
| `cargo test -p melee-ft -p melee-ef -p melee-cmd -p melee-coll -p hsd-gobj` | 144 passed: 90 ft, 5 ef, 6 cmd, 7 coll, 36 gobj |
| `cargo test -p melee-sim --test alloc_gate -- --nocapture` | 5 passed with the lower ceilings above |
| `tools/check-release-math.sh` | PASS: 74 tests across debug/release and all six optimization levels |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo fmt --all`, then `cargo fmt --all -- --check` | PASS |
| `tools/perf-gate.sh` | PASS at unchanged default tolerances; docs/PERF.md retains both C8 runs |
| `tools/merge-check.sh lane/combat` | BLOCKED at ancestry guard, before its build/test chain |

The performance tool ran with `CARGO_HOME=/private/tmp/c13-cargo-home`, an
available writable registry cache. No tolerance environment variables were set.
This avoids a sandbox-denied global registry unpack; it changes no source,
profile, compiler flags or gate thresholds.

Merge-check printed exactly:

```text
[PASS] data: no tracked game data or protected lane changes
[PASS] data: oracle traces present
[FAIL] rebase: main is not an ancestor of lane/combat (or ref lookup failed)
```

The lane remains at `97b2c264190910995bfdb1b39458c5d3341ac774`; main was
`664b762` when checked. The user forbids Git writes, so no rebase, checkout,
commit or bypass was attempted. Standalone strict build and test commands passed,
but this is not a green merge-check or validation of a future merged tree.

### Exact final test lines

These are copied from the last complete workspace runs after the size fix.
Multi-target package totals above are sums, not invented Cargo summary lines.
Full command output remains in `/private/tmp/c8-final-*.log`; below are the
nonempty result lines for the requested package suites and M4/M5. The two full
workspace logs are `c8-final-gate-debug.log` and `c8-final-gate-release.log`.

debug (`cargo gate`):

```text
hsd_gobj::unit: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
borrowed_dispatch: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
scheduler: test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
hsd_particle::unit: test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
lifecycle: test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
live_bf_idle: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.85s
live_bf_start: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.05s
live_dl_idle: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
live_dl_start: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.49s
live_fd: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.24s
live_fd_airdodge: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.16s
live_fd_dash: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s
live_fd_jab_marth: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.14s
live_fd_jump: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.17s
live_fd_ledge: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.60s
live_fd_roll: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.07s
live_fd_shield: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.06s
live_fd_spotdodge: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.12s
live_fd_start: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.58s
live_fd_wavedash: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.16s
live_grab_fd_marth: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.25s
live_jab_fd_fox: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.15s
live_ko_fd_marth: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.02s
live_shieldhit_fd_marth: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.11s
live_tech_fd_marth: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.15s
live_utilt_fd_marth: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.16s
live_ys_idle: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
live_ys_start: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.58s
opcodes: test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
real_fd_bank: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
real_fd_particles: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
start_paths: test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
interpreter: test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
melee_coll::unit: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
colliders: test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
melee_ef::unit: test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
melee_ft::unit: test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
airborne_ref_oracle: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
animation_desc: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
animation_ref_oracle: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.09s
bone_desc: test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
common_desc: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
cpu_initialization: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
fighter_animation: test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
fighter_attributes: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
fighter_graphics: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
fox_spawn_native: test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
grounded_physics_native: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.75s
human_idle_input_600: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
human_input: test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
idle_fox_600: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.05s
idle_ground_fields_600: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.43s
input_oracle: test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
movement_fox_states: test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.40s
real_fox_data: test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
real_fox_wait_playback: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.21s
start_fox_600: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.14s
start_fox_bones_130: test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.97s
start_fox_states: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
m4_gate: test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 84.45s
m5_gate: test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.29s
```

release (`cargo gate --release`):

```text
hsd_gobj::unit: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
borrowed_dispatch: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
scheduler: test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
hsd_particle::unit: test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
lifecycle: test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
live_bf_idle: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.02s
live_bf_start: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.79s
live_dl_idle: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
live_dl_start: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
live_fd: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.68s
live_fd_airdodge: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
live_fd_dash: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
live_fd_jab_marth: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
live_fd_jump: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
live_fd_ledge: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s
live_fd_roll: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
live_fd_shield: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
live_fd_spotdodge: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
live_fd_start: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.71s
live_fd_wavedash: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
live_grab_fd_marth: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.39s
live_jab_fd_fox: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
live_ko_fd_marth: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
live_shieldhit_fd_marth: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
live_tech_fd_marth: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
live_utilt_fd_marth: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
live_ys_idle: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
live_ys_start: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
opcodes: test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
real_fd_bank: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
real_fd_particles: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
start_paths: test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
interpreter: test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
melee_coll::unit: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
colliders: test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
melee_ef::unit: test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
melee_ft::unit: test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
airborne_ref_oracle: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
animation_desc: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
animation_ref_oracle: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
bone_desc: test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
common_desc: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
cpu_initialization: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
fighter_animation: test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
fighter_attributes: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
fighter_graphics: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
fox_spawn_native: test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
grounded_physics_native: test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.55s
human_idle_input_600: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
human_input: test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
idle_fox_600: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
idle_ground_fields_600: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
input_oracle: test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
movement_fox_states: test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s
real_fox_data: test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
real_fox_wait_playback: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
start_fox_600: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
start_fox_bones_130: test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.20s
start_fox_states: test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
m4_gate: test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.96s
m5_gate: test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.75s
```

Allocation census:

```text
start_fd_fox: 599 measured ticks; simulate-only 17 (0.028381/tick), peak 5, allocating ticks 9; with snapshot 74892 (125.028381/tick), peak 130; snapshot overhead 74875 (125.000000/tick)
start_bf_fox: 599 measured ticks; simulate-only 17 (0.028381/tick), peak 6, allocating ticks 11; with snapshot 74892 (125.028381/tick), peak 131; snapshot overhead 74875 (125.000000/tick)
idle_fd_fox: 599 measured ticks; simulate-only 1 (0.001669/tick), peak 1, allocating ticks 1; with snapshot 74876 (125.001669/tick), peak 126; snapshot overhead 74875 (125.000000/tick)
jab_fd_marth: 299 measured ticks; simulate-only 12 (0.040134/tick), peak 2, allocating ticks 10; with snapshot 37387 (125.040134/tick), peak 127; snapshot overhead 37375 (125.000000/tick)
ko_fd_marth: 479 measured ticks; simulate-only 690 (1.440501/tick), peak 668, allocating ticks 14; with snapshot 60565 (126.440501/tick), peak 793; snapshot overhead 59875 (125.000000/tick)
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.02s
```

Release math:

```text
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.41s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Fused math: opt-level=0
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Fused math: opt-level=1
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Fused math: opt-level=2
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Fused math: opt-level=3
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Fused math: opt-level=s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
Fused math: opt-level=z
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Performance gate final line:

```text
[PASS] perf-gate: 3864176 stripped bytes, 3424256 text bytes; load 169.320 ms; ticks_600 23.606 ms
```

## Changed files

Deleted files are included: their implementations moved to the owning crates.
The pre-existing decomp symlink type-change is excluded. Existing integration
fixtures changed only for imports, ownership access, and fixed-vector construction;
`alloc_gate` also lowers its five ceilings.

```text
CLAUDE.md
Cargo.lock
Cargo.toml
TRACKER.md
crates/ft-yoshi/Cargo.toml
crates/ft-yoshi/src/init.rs
crates/ft-yoshi/src/shield.rs
crates/gekko-math/src/lib.rs
crates/gekko-math/src/matrix.rs
crates/hsd-anim/src/jobj.rs
crates/hsd-anim/src/mtx.rs
crates/hsd-gobj/src/world.rs
crates/hsd-gobj/tests/borrowed_dispatch.rs
crates/hsd-types/src/lib.rs
crates/melee-cmd/Cargo.toml
crates/melee-cmd/src/decode.rs
crates/melee-cmd/src/lib.rs
crates/melee-cmd/src/state.rs
crates/melee-cmd/tests/interpreter.rs
crates/melee-coll/Cargo.toml
crates/melee-coll/src/damage.rs
crates/melee-coll/src/defense.rs
crates/melee-coll/src/detection.rs
crates/melee-coll/src/geometry.rs
crates/melee-coll/src/hitbox.rs
crates/melee-coll/src/hurtbox.rs
crates/melee-coll/src/lib.rs
crates/melee-coll/src/overlap.rs
crates/melee-coll/tests/colliders.rs
crates/melee-ef/src/fixed.rs
crates/melee-ef/src/lib.rs
crates/melee-ef/src/request.rs
crates/melee-ft/Cargo.toml
crates/melee-ft/src/anim/playback.rs
crates/melee-ft/src/fighter/assets.rs
crates/melee-ft/src/fighter/caches.rs
crates/melee-ft/src/fighter/commands.rs
crates/melee-ft/src/fighter/damage.rs
crates/melee-ft/src/fighter/down.rs
crates/melee-ft/src/fighter/effects.rs
crates/melee-ft/src/fighter/escape.rs
crates/melee-ft/src/fighter/grab.rs
crates/melee-ft/src/fighter/grab_throw.rs
crates/melee-ft/src/fighter/hitbox.rs
crates/melee-ft/src/fighter/life.rs
crates/melee-ft/src/fighter/mod.rs
crates/melee-ft/src/fighter/overlap.rs
crates/melee-ft/src/fighter/procs.rs
crates/melee-ft/src/fighter/shield.rs
crates/melee-ft/src/fighter/smash.rs
crates/melee-ft/src/fighter/spawn.rs
crates/melee-ft/tests/fighter_animation.rs
crates/melee-ft/tests/fighter_graphics.rs
crates/melee-ft/tests/fighter_support/mod.rs
crates/melee-ft/tests/fighter_support/replay.rs
crates/melee-lb/src/collision.rs
crates/melee-lb/src/lib.rs
crates/melee-sim/Cargo.toml
crates/melee-sim/src/frame.rs
crates/melee-sim/src/frame/combat.rs
crates/melee-sim/src/frame/falco_bones.rs
crates/melee-sim/src/frame/falcon_bones.rs
crates/melee-sim/src/frame/fall_states.rs
crates/melee-sim/src/frame/marth_bones.rs
crates/melee-sim/src/frame/peach_bones.rs
crates/melee-sim/src/frame/puff_bones.rs
crates/melee-sim/src/frame/puff_state.rs
crates/melee-sim/src/frame/rendered_pose.rs
crates/melee-sim/src/frame/yoshi_bones.rs
crates/melee-sim/src/frame/yoshi_state.rs
crates/melee-sim/src/initial_state/fighter.rs
crates/melee-sim/tests/alloc_gate.rs
crates/melee-types/src/combat.rs
crates/melee-types/src/fixed.rs
crates/melee-types/src/lib.rs
docs/PERF.md
docs/PORT_NOTES/C8_CMD_COLL.md
```
