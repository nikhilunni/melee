# C2 concrete fighter core and C7 descriptor capabilities

2026-09-09, combat lane. Implemented and validated against the existing failing baseline;
left uncommitted as requested. This report distinguishes debug oracle acceptance from
release size/IR measurements. The separate C14 release-exactness task is unchanged.

## Split and operation boundaries

`Fighter<C>` owns `core: FighterCore`, `character: C`, and the installed
`motion_row: MotionRow<C>`. `Deref`/`DerefMut` retain ordinary field and method access.
The concrete core owns physics, environment collision, animation trees, script state,
input, timers, damage/shield bookkeeping, hit/hurt capsules, effects, dynamics,
player mirrors, camera state and raw snapshot emission. Its scalar `MotionState`
contains action/semantic IDs and animation/coverage metadata, with no typed callbacks.
`install_motion_row` installs the scalar state and typed row together for import;
ordinary motion changes install them around the concrete reset at the original entry
boundary, before the next hook. The row remains installed, rather than being re-looked-up
from an action ID at dispatch. The existing callback-substitution test still proves that.

The generic shell retains common/special table selection, typed row callbacks,
character hooks and the ordered state-transition graph. Initial owner construction,
large motion-reset/playback bodies, timer/input processing, shield/launch arithmetic,
throw geometry, per-capsule hit detection and dynamics solving are concrete.
Pair helpers that need neither fighter's hooks now accept `FighterCore` references,
removing the seven-by-seven character product. Helpers that enter a state retain small
generic callers around their concrete calculation stages.

All 61 methods moved wholesale have identical normalized bodies (comments, whitespace
and explicit `.core` field qualification excluded). Larger extractions preserve hook
boundaries: eligible-hit filtering → hurtbox hook → concrete collision; launch arithmetic
→ motion entry → playback → hitstun bookkeeping; throw preparation → each fighter's
motion entry → pose setup; gravity → multijump attributes → drift; dynamics inputs sampled
once → each set's force-bone hook → that set's concrete solve. Floating-point expressions,
FMA helpers, widths, RNG calls and test expectations were not changed. Construction uses
the asset kind already validated against the character kind as immutable descriptor data.
No hook-free body required changing floating-point or hook operation order.

The core still uses the existing allocations; C5 remains separate. No dynamic dispatch,
unsafe code, new per-tick allocation or family-crate dependency was introduced.
SnapshotSink and EffectSink interfaces retain their existing dispatch conventions.

C12 import changes are restricted to core-reference/borrow call sites, the installed-row
setter, and making the saved-pose reader accept the core. C13's saved-pose support similarly
changes only parameter types; its reader body and all oracle values remain unchanged.
The protected idle/real-wait/start test files themselves are untouched.

## Measurement method and limits

This checkout has seven loaded character implementations (49 ordered pairs); 26 is the
future roster, not the measured instantiation count. `ft-mario` is an additional workspace
package but is not a loaded scene character. Neither `docs/PERF.md` nor
`tools/perf-gate.sh` exists on this branch, so no PERF block could be run.

The exact requested `cargo llvm-lines -p melee-sim --release` exits 101 both before and
after: the package has a library and binary, and rustc requires one target. The successful
commands are:

```sh
cargo llvm-lines -p melee-sim --release --bin melee-sim
cargo llvm-lines -p melee-sim --release --lib
cargo llvm-lines -p melee-ft --release --lib
```

The binary's IR is CLI code; the library contains the fighter instantiations. The melee-ft
library is also measured because extracted concrete definitions live there. Before samples
were captured from the original source; the two library baselines use a temporary source
copy restored to that same pre-edit state. Tool: cargo-llvm-lines 0.4.48. All measurements
use the branch's unchanged release profile and rustflags. IR counts are compiler-emitted
lines/definitions, not linked machine-code instructions. Crate-owned totals include symbols
starting `melee_ft::` or `<melee_ft::`, excluding standard-library generic helpers merely
parameterized by a melee-ft type.

For both build timings, clean exactly these release packages, then time the build with a
monotonic wall clock. Retain dependency caches, as requested; this is a scoped clean build,
not an empty Cargo target directory. Build sequentially, copy the binary and run `strip` on
the copy. One sample per revision; wall times include machine load and are not a statistical
throughput benchmark.

```sh
cargo clean -p melee-sim -p melee-ft -p ft-fox -p ft-mars -p ft-falco \
  -p ft-captain -p ft-peach -p ft-yoshi -p ft-purin -p ft-mario --release
cargo build --release -p melee-sim
cp target/release/melee-sim /private/tmp/c2-final/melee-sim
strip /private/tmp/c2-final/melee-sim
```

| Measurement | Before | After | Change |
|---|---:|---:|---:|
| Stripped release bytes | 4,204,168 | 3,696,072 | -12.09% |
| Scoped clean release build, wall seconds | 10.972 | 8.157 | -25.66% |

The task supplied an older 3,866,720-byte baseline. The table uses the actual local pre-edit
binary for a like-for-like comparison; release profile settings were not changed.

| IR target / scope | Before lines | After lines | Before copies | After copies |
|---|---:|---:|---:|---:|
| sim-bin: all | 52,267 | 52,267 | 1,647 | 1,647 |
| sim-bin: melee-ft symbols | 0 | 0 | 0 | 0 |
| sim-lib: all | 764,422 | 535,335 | 12,604 | 9,842 |
| sim-lib: melee-ft symbols | 297,472 | 138,829 | 2,811 | 1,887 |
| ft-lib: all | 154,623 | 183,804 | 2,658 | 3,183 |
| ft-lib: melee-ft symbols | 64,468 | 79,974 | 258 | 429 |
| All measured targets: melee-ft symbols | 361,940 | 218,803 | 3,069 | 2,316 |

Totals sum emitted definitions across targets; the full target-scoped census below makes
any cross-target copies explicit. They are not a deduplicated linked-symbol total.

Concrete-core census: 118 target-scoped symbols, 123 emitted copies.
All 105 named core methods/trait methods have exactly one emitted copy.
Five closure display names show two copies because llvm-lines combines distinct closures
inside the same concrete method under `{{closure}}`. These are not character instantiations:

| Closure display group | Copies | Distinct source closures |
|---|---:|---|
| `update_idle_animation::{{closure}}` | 2 | motion lookup and restart callback |
| `first_ground_transition::{{closure}}` | 2 | predicate map and transition find |
| `update_idle_animation::{{closure}}::{{closure}}` | 2 | script callback and empty part callback |
| `collision_revival::{{closure}}` | 2 | one ECB-pose closure in each collision branch |
| `start_motion_animation::{{closure}}` | 2 | borrowed-motion selection and borrowed-script selection |

Representative core extractions (IR lines / emitted copies):

| Function | Before | After |
|---|---:|---:|
| `step_animation` | 1869 / 7 | 267 / 1 |
| `land` | 392 / 7 | 56 / 1 |
| `resolve_graphics_commands` | 2856 / 7 | 408 / 1 |
| `proc_status` | 609 / 7 | 87 / 1 |
| `fall_animation` | 1099 / 7 | 157 / 1 |
| `shield_contact` | 777 / 7 | 111 / 1 |
| `record_shield_hit` | 9653 / 49 | 197 / 1 |
| `candidate` | 8526 / 49 | 173 / 1 |
| `capture_delta` | 4116 / 49 | 84 / 1 |
| `update_constraint` | 5243 / 49 | 107 / 1 |

Pair wrappers still have 49 copies because either fighter can enter a character-specific
state. Their large calculations are concrete: `detect_hit` fell from 21,511 to 4,851 IR
lines, and `release_back_throw` from 25,921 to 2,548; `detect_eligible_hit` and
`prepare_throw_release` each have one calculation-body copy.

## C7: each old kind branch becomes descriptor data

`CharacterDescriptor::common_behavior` supplies a `CommonBehavior` value built at constant
initialization with `CommonBehavior::for_kind`. Defaults read the named field; existing
character overrides continue to take precedence. Unported paths retain exactly the old
explicit `unimplemented!` behavior. A set comparison against each original branch confirms
all six kind sets are identical; no retail behavior was newly invented.

| Default hook | Descriptor field | Original selected kinds | Retail source |
|---|---|---|---|
| forward_smash_variant | forward_smash_entry | Ness, Peach, GameWatch, Pikachu, Pichu | ftCo_AttackS4.c:145-166 (8008C348) |
| throw_variant | throw_callback | Fox, Samus, Kirby, Yoshi | ftCo_Throw.c:145-157,346-353 |
| jab_variant | jab_entry | GameWatch, Pikachu, Pichu | ftCo_Attack1.c:89-110 (8008AB84 / 8008ABC0) |
| air_dodge_tether | air_dodge_tether | Link, CLink, Samus | ftCo_AirCatch.c:54-79 (800C3B10) |
| on_landing | landing_reset | Mario, DrMario, Peach, Emblem, GameWatch, Popo, Nana, Kirby, Mewtwo | ftCo_Landing.c:51-83 (800D5AEC) |
| escape_variant | morph_ball_roll | Samus; rolling only | ftCo_Escape.c:83-85 |

The task estimated about 18 checks; this branch has six default-body branch sites and
18 matching source lines after extraction. The table-only test character now explicitly
rejects runtime-kind access, which its table tests never use. The required scan returns
only descriptor initialization mappings in `fighter/assets.rs`:

```text
crates/melee-ft/src/fighter/assets.rs:65:                FighterKind::Ness
crates/melee-ft/src/fighter/assets.rs:66:                    | FighterKind::Peach
crates/melee-ft/src/fighter/assets.rs:67:                    | FighterKind::GameWatch
crates/melee-ft/src/fighter/assets.rs:68:                    | FighterKind::Pikachu
crates/melee-ft/src/fighter/assets.rs:69:                    | FighterKind::Pichu
crates/melee-ft/src/fighter/assets.rs:73:                FighterKind::Fox | FighterKind::Samus | FighterKind::Kirby | FighterKind::Yoshi
crates/melee-ft/src/fighter/assets.rs:77:                FighterKind::GameWatch | FighterKind::Pikachu | FighterKind::Pichu
crates/melee-ft/src/fighter/assets.rs:81:                FighterKind::Link | FighterKind::CLink | FighterKind::Samus
crates/melee-ft/src/fighter/assets.rs:85:                FighterKind::Mario
crates/melee-ft/src/fighter/assets.rs:86:                    | FighterKind::DrMario
crates/melee-ft/src/fighter/assets.rs:87:                    | FighterKind::Peach
crates/melee-ft/src/fighter/assets.rs:88:                    | FighterKind::Emblem
crates/melee-ft/src/fighter/assets.rs:89:                    | FighterKind::GameWatch
crates/melee-ft/src/fighter/assets.rs:90:                    | FighterKind::Popo
crates/melee-ft/src/fighter/assets.rs:91:                    | FighterKind::Nana
crates/melee-ft/src/fighter/assets.rs:92:                    | FighterKind::Kirby
crates/melee-ft/src/fighter/assets.rs:93:                    | FighterKind::Mewtwo
crates/melee-ft/src/fighter/assets.rs:95:            morph_ball_roll: matches!(kind, FighterKind::Samus),
```

## Exact acceptance results

### `cargo test -p melee-sim --test m4_gate`

Aggregate binaries/doc-tests reached: 261 passed / 0 failed / 0 ignored.

```text
test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 284.60s
```

No new failing binaries compared with the captured baseline.

### `cargo test -p melee-sim --test m5_gate`

Aggregate binaries/doc-tests reached: 6 passed / 2 failed / 0 ignored.

```text
test result: FAILED. 6 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.43s
error: test failed, to rerun pass `-p melee-sim --test m5_gate`
```

No new failing binaries compared with the captured baseline.

### `cargo test -p hsd-particle --no-fail-fast`

Aggregate binaries/doc-tests reached: 63 passed / 12 failed / 0 ignored.

```text
Running tests/live_bf_idle.rs (target/debug/deps/live_bf_idle-fea824b16b9c7543)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.61s
Running tests/live_dl_idle.rs (target/debug/deps/live_dl_idle-f8637d625a6a6db5)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
Running tests/live_fd.rs (target/debug/deps/live_fd-c68dcb718e95abed)
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
Running tests/live_fd_jab_marth.rs (target/debug/deps/live_fd_jab_marth-cc0c27399e234fc5)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.41s
Running tests/live_fd_ledge.rs (target/debug/deps/live_fd_ledge-b24e5bd0a4ac5856)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.48s
Running tests/live_fd_start.rs (target/debug/deps/live_fd_start-4f8109f3a14bafaf)
test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.46s
Running tests/live_jab_fd_fox.rs (target/debug/deps/live_jab_fd_fox-dcee3d6a81e15500)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.86s
Running tests/live_ko_fd_marth.rs (target/debug/deps/live_ko_fd_marth-3b0dd151ee66136a)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.23s
Running tests/live_utilt_fd_marth.rs (target/debug/deps/live_utilt_fd_marth-fb7b5bcf7752aae4)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.36s
Running tests/live_ys_idle.rs (target/debug/deps/live_ys_idle-c3f8ed14d2a7d0ad)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
Running tests/live_ys_start.rs (target/debug/deps/live_ys_start-aa617242786dba9b)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.58s
error: test failed, to rerun pass `-p hsd-particle --test live_bf_idle`
error: test failed, to rerun pass `-p hsd-particle --test live_dl_idle`
error: test failed, to rerun pass `-p hsd-particle --test live_fd`
error: test failed, to rerun pass `-p hsd-particle --test live_fd_jab_marth`
error: test failed, to rerun pass `-p hsd-particle --test live_fd_ledge`
error: test failed, to rerun pass `-p hsd-particle --test live_fd_start`
error: test failed, to rerun pass `-p hsd-particle --test live_jab_fd_fox`
error: test failed, to rerun pass `-p hsd-particle --test live_ko_fd_marth`
error: test failed, to rerun pass `-p hsd-particle --test live_utilt_fd_marth`
error: test failed, to rerun pass `-p hsd-particle --test live_ys_idle`
error: test failed, to rerun pass `-p hsd-particle --test live_ys_start`
error: 11 targets failed:
    `-p hsd-particle --test live_bf_idle`
    `-p hsd-particle --test live_dl_idle`
    `-p hsd-particle --test live_fd`
    `-p hsd-particle --test live_fd_jab_marth`
    `-p hsd-particle --test live_fd_ledge`
    `-p hsd-particle --test live_fd_start`
    `-p hsd-particle --test live_jab_fd_fox`
    `-p hsd-particle --test live_ko_fd_marth`
    `-p hsd-particle --test live_utilt_fd_marth`
    `-p hsd-particle --test live_ys_idle`
    `-p hsd-particle --test live_ys_start`
```

No new failing binaries compared with the captured baseline.

### `cargo test -p melee-ft --no-fail-fast`

Aggregate binaries/doc-tests reached: 85 passed / 5 failed / 0 ignored.

```text
Running tests/idle_fox_600.rs (target/debug/deps/idle_fox_600-fb28c825fff5a419)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.41s
Running tests/real_fox_wait_playback.rs (target/debug/deps/real_fox_wait_playback-a75c1913bb599165)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s
Running tests/start_fox_600.rs (target/debug/deps/start_fox_600-9fbba489e085a541)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.22s
Running tests/start_fox_bones_130.rs (target/debug/deps/start_fox_bones_130-69426dff17b5adb4)
test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.96s
Running tests/start_fox_states.rs (target/debug/deps/start_fox_states-ffcfba60a5d3f069)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.04s
error: test failed, to rerun pass `-p melee-ft --test idle_fox_600`
error: test failed, to rerun pass `-p melee-ft --test real_fox_wait_playback`
error: test failed, to rerun pass `-p melee-ft --test start_fox_600`
error: test failed, to rerun pass `-p melee-ft --test start_fox_bones_130`
error: test failed, to rerun pass `-p melee-ft --test start_fox_states`
error: 5 targets failed:
    `-p melee-ft --test idle_fox_600`
    `-p melee-ft --test real_fox_wait_playback`
    `-p melee-ft --test start_fox_600`
    `-p melee-ft --test start_fox_bones_130`
    `-p melee-ft --test start_fox_states`
```

No new failing binaries compared with the captured baseline.

### `cargo gate`

Aggregate binaries/doc-tests reached: 305 passed / 1 failed / 0 ignored.

```text
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.04s
error: test failed, to rerun pass `-p hsd-particle --test live_bf_idle`
```

No new failing binaries compared with the captured baseline.


M5 retains exactly `jab_fd_fox_300_ticks_and_ordered_particle_draws` (ef particle 0/6)
and `jab_fd_marth_300_ticks_and_ordered_particle_draws` (stale aggregate). The workspace
`cargo gate` stops at the same first failing target, `hsd-particle::live_bf_idle`;
the two `--no-fail-fast` runs enumerate the complete requested fighter/particle failures.
No expected values, ignored tests, gate definitions or comparisons were loosened.

Strict clippy and formatting:

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 25.44s
cargo fmt --all: exit 0
cargo fmt --all -- --check: exit 0
git diff --check: exit 0
```

## File list and untouched boundaries

```text
TRACKER.md
crates/ft-captain/src/init.rs
crates/ft-falco/src/init.rs
crates/ft-fox/src/init.rs
crates/ft-mars/src/init.rs
crates/ft-peach/src/init.rs
crates/ft-purin/src/init.rs
crates/ft-yoshi/src/init.rs
crates/ft-yoshi/src/shield.rs
crates/melee-ft/src/fighter/air_dodge.rs
crates/melee-ft/src/fighter/assets.rs
crates/melee-ft/src/fighter/attack.rs
crates/melee-ft/src/fighter/damage.rs
crates/melee-ft/src/fighter/dash.rs
crates/melee-ft/src/fighter/down.rs
crates/melee-ft/src/fighter/dynamic_commands.rs
crates/melee-ft/src/fighter/effects.rs
crates/melee-ft/src/fighter/entry.rs
crates/melee-ft/src/fighter/escape.rs
crates/melee-ft/src/fighter/fall.rs
crates/melee-ft/src/fighter/grab.rs
crates/melee-ft/src/fighter/grab_throw.rs
crates/melee-ft/src/fighter/jump.rs
crates/melee-ft/src/fighter/landing.rs
crates/melee-ft/src/fighter/ledge.rs
crates/melee-ft/src/fighter/life.rs
crates/melee-ft/src/fighter/mod.rs
crates/melee-ft/src/fighter/multi_jump.rs
crates/melee-ft/src/fighter/overlap.rs
crates/melee-ft/src/fighter/pass.rs
crates/melee-ft/src/fighter/procs.rs
crates/melee-ft/src/fighter/run.rs
crates/melee-ft/src/fighter/shield.rs
crates/melee-ft/src/fighter/smash.rs
crates/melee-ft/src/fighter/snapshot.rs
crates/melee-ft/src/fighter/spawn.rs
crates/melee-ft/src/fighter/squat.rs
crates/melee-ft/src/fighter/state.rs
crates/melee-ft/src/fighter/state/callbacks/animation.rs
crates/melee-ft/src/fighter/state/callbacks/camera.rs
crates/melee-ft/src/fighter/state/callbacks/collision.rs
crates/melee-ft/src/fighter/state/callbacks/input.rs
crates/melee-ft/src/fighter/state/callbacks/physics.rs
crates/melee-ft/src/fighter/state/row.rs
crates/melee-ft/src/fighter/state/special.rs
crates/melee-ft/src/fighter/turn.rs
crates/melee-ft/src/fighter/turn_run.rs
crates/melee-ft/src/fighter/walk.rs
crates/melee-ft/tests/fighter_support/mod.rs
crates/melee-ft/tests/fighter_support/saved_pose.rs
crates/melee-ft/tests/fox_spawn_native.rs
crates/melee-sim/src/frame.rs
crates/melee-sim/src/initial_state/fighter.rs
crates/melee-sim/src/initial_state/saved_pose.rs
crates/melee-sim/src/scene_stage/pupupu.rs
docs/PORT_NOTES/C2_CONCRETE_CORE.md
```

No commits or Git write commands were run. The pre-existing decomp symlink/type change
is unchanged. No harness/roms, harness/traces, harness/scenarios, decomp or fixture data was
modified. Missing PERF tooling and the pre-existing oracle failures are the only acceptance
limits; release oracle correctness remains C14 rather than a claim of this measurement.

## Generic remainder, with a reason for every function

The entries name immediate dependencies; following a state-transition call reaches
`change_motion_state_with_options` and its character hooks/table selection. Typed row
wrappers deliberately retain their phase-specific ABI even when they delegate to the core.
Trait defaults and table constructors are generic dispatch/data plumbing. `Deref`,
`DerefMut`, `install_motion_row`, typed `MotionRow` Copy/Clone/Debug, and the Snapshot
forwarder are small compatibility adapters. Effect-sink generics specialize by sink type,
not character; dynamics floor solving now uses concrete optional map data.

| Function | Why generic |
|---|---|
| `air_dodge.rs::air_dodge_animation` | Character-aware call: `enter_air_dodge_fall` |
| `air_dodge.rs::air_dodge_input` | Character hook/data: `air_dodge_tether` |
| `air_dodge.rs::air_dodge_physics` | Character-aware call: `airborne_physics` |
| `air_dodge.rs::enter_air_dodge` | Character-aware call: `change_motion_state` |
| `attack.rs::enter_ground_attack` | Character hook/data: `jab_variant` |
| `attack.rs::jab_animation` | Character-aware call: `change_motion_state` |
| `attack.rs::jab_input` | Character-aware call: `apply_ground_transition` |
| `attack.rs::tilt_input` | Character-aware call: `apply_ground_transition` |
| `damage.rs::begin_damage_reaction` | Character-aware call: `change_motion_state` |
| `damage.rs::damage_animation` | Character-aware call: `change_motion_state` |
| `damage.rs::damage_collision` | Character-aware call: `enter_down_bound`, `enter_landing`, `try_tech` |
| `damage.rs::damage_input` | Character hook/data: `check_float_input` |
| `damage.rs::damage_physics` | Character-aware call: `airborne_physics` |
| `damage.rs::detect_hit` | Character hook/data: `check_hurtbox_interaction` |
| `damage.rs::process_damage` | Character-aware call: `begin_damage_reaction` |
| `dash.rs::dash_animation` | Character-aware call: `change_motion_state` |
| `dash.rs::dash_input` | Character-aware call: `enter_run`, `try_redash` |
| `dash.rs::enter_dash` | Character-aware call: `change_motion_state` |
| `dash.rs::try_redash` | Character-aware call: `enter_dash`, `enter_turn` |
| `down.rs::down_animation` | Character-aware call: `change_motion_state` |
| `down.rs::enter_down_bound` | Character-aware call: `change_motion_state` |
| `down.rs::try_tech` | Character-aware call: `change_motion_state` |
| `entry.rs::enter_match` | Character-aware call: `change_motion_state` |
| `entry.rs::entry_animation` | Character-aware call: `change_motion_state` |
| `escape.rs::enter_escape` | Character hook/data: `escape_variant` |
| `escape.rs::escape_animation` | Character hook/data: `escape_animated`, `escape_finished` |
| `fall.rs::enter_air_dodge_fall` | Character-aware call: `change_motion_state` |
| `fall.rs::land_from_special_fall` | Character-aware call: `change_motion_state`, `enter_special_landing` |
| `fall.rs::special_fall_physics` | Character-aware call: `airborne_physics` |
| `grab.rs::capture_collision` | Character-aware call: `catch_collision` |
| `grab.rs::capture_pair` | Character hook/data: `catch_variant` |
| `grab.rs::capture_wait` | Character-aware call: `change_motion_state` |
| `grab.rs::catch_animation` | Character-aware call: `change_motion_state` |
| `grab.rs::catch_collision` | Character-aware call: `change_motion_state` |
| `grab.rs::enter_catch` | Character hook/data: `catch_variant` |
| `grab.rs::enter_catch_wait` | Character-aware call: `change_motion_state` |
| `grab_throw.rs::enter_back_throw` | Character hook/data: `throw_variant` |
| `grab_throw.rs::release_back_throw` | Character-aware call: `begin_damage_reaction` |
| `jump.rs::airborne_physics` | Character hook/data: `aerial_jump_style` |
| `jump.rs::enter_aerial_jump` | Character hook/data: `aerial_jump_entered`, `aerial_jump_style` |
| `jump.rs::enter_knee_bend` | Character-aware call: `change_motion_state` |
| `jump.rs::jump_animation` | Character hook/data: `aerial_jump_animated` |
| `jump.rs::knee_bend_animation` | Character-aware call: `change_motion_state` |
| `landing.rs::enter_landing` | Character hook/data: `on_landing` |
| `landing.rs::enter_landing_squat` | Character-aware call: `change_motion_state` |
| `landing.rs::enter_special_landing` | Character hook/data: `on_landing` |
| `landing.rs::landing_animation` | Character-aware call: `change_motion_state` |
| `ledge.rs::cliff_climb_animation` | Character-aware call: `change_motion_state` |
| `ledge.rs::cliff_climb_physics` | Character-aware call: `ledge_physics` |
| `ledge.rs::enter_cliff_catch` | Character-aware call: `change_motion_state`, `ledge_physics` |
| `ledge.rs::enter_cliff_option` | Character-aware call: `change_motion_state` |
| `ledge.rs::ledge_animation` | Character-aware call: `change_motion_state` |
| `ledge.rs::ledge_collision` | Character-aware call: `change_motion_state`, `enter_landing` |
| `ledge.rs::ledge_input` | Character-aware call: `change_motion_state`, `enter_cliff_option` |
| `ledge.rs::ledge_jump_physics` | Character-aware call: `airborne_physics` |
| `ledge.rs::ledge_physics` | Character-aware call: `change_motion_state` |
| `ledge.rs::try_grab_ledge` | Character-aware call: `enter_cliff_catch` |
| `life.rs::check_blast_zone` | Character-aware call: `change_motion_state` |
| `life.rs::enter_revival` | Character-aware call: `change_motion_state` |
| `life.rs::reset_for_revival` | Character hook/data: `from_archive`, `spawn` |
| `life.rs::revival_animation` | Character-aware call: `change_motion_state` |
| `multi_jump.rs::aerial_jump_requested` | Character hook/data: `multi_jump_attributes` |
| `multi_jump.rs::enter_multi_jump` | Character hook/data: `multi_jump_attributes`, `multi_jump_family` |
| `multi_jump.rs::multi_jump_animation` | Character-aware call: `change_motion_state`, `multi_jump_turn` |
| `multi_jump.rs::multi_jump_physics` | Character hook/data: `multi_jump_attributes` |
| `multi_jump.rs::multi_jump_turn` | Character hook/data: `multi_jump_attributes` |
| `pass.rs::enter_pass` | Character-aware call: `change_motion_state` |
| `procs.rs::proc_anim` | Dispatches the installed typed row at this scheduler phase |
| `procs.rs::proc_camera` | Dispatches the installed typed row at this scheduler phase |
| `procs.rs::proc_camera_with_map` | Dispatches the installed typed row at this scheduler phase |
| `procs.rs::proc_dynamics` | Dispatches the installed typed row at this scheduler phase |
| `procs.rs::proc_dynamics_with_map` | Dispatches the installed typed row at this scheduler phase |
| `procs.rs::proc_input` | Dispatches the installed typed row at this scheduler phase |
| `procs.rs::proc_map` | Dispatches the installed typed row at this scheduler phase |
| `procs.rs::proc_map_with_assets` | Dispatches the installed typed row at this scheduler phase |
| `procs.rs::proc_process_hit` | Dispatches the installed typed row at this scheduler phase |
| `procs.rs::proc_update` | Dispatches the installed typed row at this scheduler phase |
| `procs.rs::solve_dynamics` | Character hook/data: `dynamics_first_force_bone` |
| `run.rs::enter_run` | Character-aware call: `change_motion_state` |
| `run.rs::enter_run_brake` | Character-aware call: `change_motion_state` |
| `run.rs::run_brake_animation` | Character-aware call: `change_motion_state` |
| `run.rs::run_brake_input` | Character-aware call: `enter_squat`, `try_turn_run` |
| `run.rs::run_input` | Character-aware call: `enter_run_brake`, `try_turn_run` |
| `shield.rs::enter_guard_hold` | Character hook/data: `enter_guard_hold` |
| `shield.rs::enter_guard_off` | Character hook/data: `enter_guard_off` |
| `shield.rs::enter_shield` | Character hook/data: `enter_shield`, `guard_variant` |
| `shield.rs::shield_animation` | Character hook/data: `animate_shield` |
| `shield.rs::shield_input` | Character hook/data: `input_shield` |
| `shield.rs::shield_proc` | Character-aware call: `take_shield_hit` |
| `shield.rs::take_shield_hit` | Character hook/data: `guard_variant` |
| `smash.rs::enter_forward_smash` | Character hook/data: `forward_smash_variant` |
| `spawn.rs::change_motion_state` | Character-aware call: `change_motion_state_at` |
| `spawn.rs::change_motion_state_at` | Character-aware call: `change_motion_state_with_rate` |
| `spawn.rs::change_motion_state_with_options` | Character hook/data: `animated_shield`, `on_grounded_motion` |
| `spawn.rs::change_motion_state_with_rate` | Character-aware call: `change_motion_state_with_options` |
| `spawn.rs::change_motion_state_with_source` | Character-aware call: `change_motion_state_with_options` |
| `spawn.rs::create` | Character hook/data: `on_reset`, `prepare` |
| `spawn.rs::prepare` | Typed constructor, table dispatch or thin core adapter |
| `spawn.rs::spawn` | Character hook/data: `create` |
| `spawn.rs::spawn_for_match` | Character hook/data: `create` |
| `squat.rs::enter_squat` | Character-aware call: `change_motion_state` |
| `squat.rs::squat_animation` | Character-aware call: `change_motion_state` |
| `squat.rs::squat_input` | Character-aware call: `apply_ground_transition`, `change_motion_state`, `enter_pass` |
| `state/callbacks/animation.rs::capture` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/animation.rs::catch` | Typed MotionRow callback ABI; dispatches `catch_animation` |
| `state/callbacks/animation.rs::catch_pull` | Typed MotionRow callback ABI; dispatches `enter_catch_wait` |
| `state/callbacks/animation.rs::cliff_catch` | Typed MotionRow callback ABI; dispatches `ledge_animation` |
| `state/callbacks/animation.rs::cliff_climb` | Typed MotionRow callback ABI; dispatches `cliff_climb_animation` |
| `state/callbacks/animation.rs::damage` | Typed MotionRow callback ABI; dispatches `damage_animation` |
| `state/callbacks/animation.rs::dash` | Typed MotionRow callback ABI; dispatches `dash_animation` |
| `state/callbacks/animation.rs::dead` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/animation.rs::down_bound` | Typed MotionRow callback ABI; dispatches `down_animation` |
| `state/callbacks/animation.rs::entry` | Typed MotionRow callback ABI; dispatches `entry_animation` |
| `state/callbacks/animation.rs::escape` | Typed MotionRow callback ABI; dispatches `escape_animation` |
| `state/callbacks/animation.rs::escape_air` | Typed MotionRow callback ABI; dispatches `air_dodge_animation` |
| `state/callbacks/animation.rs::fall` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/animation.rs::guard_on` | Typed MotionRow callback ABI; dispatches `shield_animation` |
| `state/callbacks/animation.rs::jab` | Typed MotionRow callback ABI; dispatches `jab_animation` |
| `state/callbacks/animation.rs::knee_bend` | Typed MotionRow callback ABI; dispatches `knee_bend_animation` |
| `state/callbacks/animation.rs::landing` | Typed MotionRow callback ABI; dispatches `landing_animation` |
| `state/callbacks/animation.rs::multi_jump` | Typed MotionRow callback ABI; dispatches `multi_jump_animation` |
| `state/callbacks/animation.rs::pass` | Typed MotionRow callback ABI; dispatches `jump_animation` |
| `state/callbacks/animation.rs::revival` | Typed MotionRow callback ABI; dispatches `revival_animation` |
| `state/callbacks/animation.rs::run` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/animation.rs::run_brake` | Typed MotionRow callback ABI; dispatches `run_brake_animation` |
| `state/callbacks/animation.rs::squat` | Typed MotionRow callback ABI; dispatches `squat_animation` |
| `state/callbacks/animation.rs::squat_wait` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/animation.rs::tech_roll` | Typed MotionRow callback ABI; dispatches `change_motion_state` |
| `state/callbacks/animation.rs::throw` | Typed MotionRow callback ABI; dispatches `jab_animation` |
| `state/callbacks/animation.rs::thrown` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/animation.rs::turn` | Typed MotionRow callback ABI; dispatches `turn_animation` |
| `state/callbacks/animation.rs::turn_run` | Typed MotionRow callback ABI; dispatches `turn_run_animation` |
| `state/callbacks/animation.rs::wait` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/animation.rs::walk` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/camera.rs::cliff` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/camera.rs::follow_fighter` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/collision.rs::capture` | Typed MotionRow callback ABI; dispatches `capture_collision` |
| `state/callbacks/collision.rs::catch` | Typed MotionRow callback ABI; dispatches `catch_collision` |
| `state/callbacks/collision.rs::cliff_catch` | Typed MotionRow callback ABI; dispatches `ledge_collision` |
| `state/callbacks/collision.rs::cliff_climb` | Typed MotionRow callback ABI; dispatches `ledge_collision` |
| `state/callbacks/collision.rs::damage` | Typed MotionRow callback ABI; dispatches `damage_collision` |
| `state/callbacks/collision.rs::entry` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/collision.rs::escape` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/collision.rs::escape_air` | Typed MotionRow callback ABI; dispatches `enter_special_landing` |
| `state/callbacks/collision.rs::fall` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/collision.rs::fall_collision` | Typed MotionRow callback ABI; dispatches `change_motion_state`, `enter_landing`, `land_from_special_fall`, `try_grab_ledge` |
| `state/callbacks/collision.rs::fall_special` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/collision.rs::finish_ground` | Typed MotionRow callback ABI; dispatches `change_motion_state` |
| `state/callbacks/collision.rs::ground_action` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/collision.rs::ground_wait` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/collision.rs::guard_set_off` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/collision.rs::jump` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/collision.rs::pass` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/collision.rs::revival` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/collision.rs::running` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/collision.rs::thrown` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/collision.rs::turn_run` | Typed MotionRow callback ABI; dispatches `turn_run_collision` |
| `state/callbacks/input.rs::aerial` | Typed MotionRow callback ABI; `check_float_input` hook |
| `state/callbacks/input.rs::catch` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/input.rs::cliff_catch` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/input.rs::cliff_climb` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/input.rs::cliff_wait` | Typed MotionRow callback ABI; dispatches `ledge_input` |
| `state/callbacks/input.rs::damage` | Typed MotionRow callback ABI; dispatches `damage_input` |
| `state/callbacks/input.rs::dash` | Typed MotionRow callback ABI; dispatches `dash_input` |
| `state/callbacks/input.rs::entry` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/input.rs::escape` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/input.rs::escape_air` | Typed MotionRow callback ABI; dispatches `air_dodge_input` |
| `state/callbacks/input.rs::escape_n` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/input.rs::fall_special` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/input.rs::guard_on` | Typed MotionRow callback ABI; dispatches `shield_input` |
| `state/callbacks/input.rs::jab` | Typed MotionRow callback ABI; dispatches `jab_input` |
| `state/callbacks/input.rs::knee_bend` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/input.rs::landing` | Typed MotionRow callback ABI; dispatches `apply_ground_transition`, `enter_landing_squat` |
| `state/callbacks/input.rs::run` | Typed MotionRow callback ABI; dispatches `run_input` |
| `state/callbacks/input.rs::run_brake` | Typed MotionRow callback ABI; dispatches `run_brake_input` |
| `state/callbacks/input.rs::squat` | Typed MotionRow callback ABI; dispatches `squat_input` |
| `state/callbacks/input.rs::tilt` | Typed MotionRow callback ABI; dispatches `tilt_input` |
| `state/callbacks/input.rs::turn` | Typed MotionRow callback ABI; dispatches `turn_input` |
| `state/callbacks/input.rs::turn_run` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/input.rs::wait` | Typed MotionRow callback ABI; dispatches `apply_ground_transition` |
| `state/callbacks/input.rs::walk` | Typed MotionRow callback ABI; dispatches `walk_input` |
| `state/callbacks/physics.rs::capture` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/physics.rs::catch` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/physics.rs::cliff_catch` | Typed MotionRow callback ABI; dispatches `ledge_physics` |
| `state/callbacks/physics.rs::cliff_climb` | Typed MotionRow callback ABI; dispatches `cliff_climb_physics` |
| `state/callbacks/physics.rs::cliff_jump2` | Typed MotionRow callback ABI; dispatches `ledge_jump_physics` |
| `state/callbacks/physics.rs::damage` | Typed MotionRow callback ABI; dispatches `damage_physics` |
| `state/callbacks/physics.rs::dead` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/physics.rs::down` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/physics.rs::entry` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/physics.rs::escape` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/physics.rs::escape_air` | Typed MotionRow callback ABI; dispatches `air_dodge_physics` |
| `state/callbacks/physics.rs::fall` | Typed MotionRow callback ABI; dispatches `airborne_physics` |
| `state/callbacks/physics.rs::fall_special` | Typed MotionRow callback ABI; dispatches `special_fall_physics` |
| `state/callbacks/physics.rs::guard_on` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/physics.rs::jab` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/physics.rs::multi_jump` | Typed MotionRow callback ABI; dispatches `multi_jump_physics` |
| `state/callbacks/physics.rs::pass` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/physics.rs::revival` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/physics.rs::running` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/physics.rs::turn_run` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/physics.rs::wait` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/callbacks/physics.rs::walk` | Typed MotionRow callback ABI; delegates calculation to FighterCore |
| `state/common_table.rs::common_table` | Typed constructor, table dispatch or thin core adapter |
| `state/row.rs::new` | Typed row construction or unsupported typed callback ABI |
| `state/row.rs::row` | Character hook/data: `action_id`, `special_rows` |
| `state/row.rs::unimplemented_anim` | Typed row construction or unsupported typed callback ABI |
| `state/row.rs::unimplemented_camera` | Typed row construction or unsupported typed callback ABI |
| `state/row.rs::unimplemented_collision` | Typed row construction or unsupported typed callback ABI |
| `state/row.rs::unimplemented_iasa` | Typed row construction or unsupported typed callback ABI |
| `state/row.rs::unimplemented_physics` | Typed row construction or unsupported typed callback ABI |
| `state/row.rs::unimplemented_row` | Typed row construction or unsupported typed callback ABI |
| `state/special.rs::enter_buffered_special` | Character hook/data: `enter_special` |
| `turn.rs::enter_turn` | Character-aware call: `change_motion_state` |
| `turn.rs::turn_animation` | Character-aware call: `change_motion_state` |
| `turn.rs::turn_input` | Character-aware call: `apply_ground_transition`, `enter_dash` |
| `turn_run.rs::enter_turn_run` | Character-aware call: `change_motion_state_at` |
| `turn_run.rs::try_turn_run` | Character-aware call: `enter_turn_run` |
| `turn_run.rs::turn_run_animation` | Character-aware call: `change_motion_state`, `enter_run` |
| `turn_run.rs::turn_run_collision` | Character-aware call: `change_motion_state` |
| `walk.rs::apply_ground_transition` | Character-aware call: `enter_buffered_special`, `enter_catch`, `enter_dash`, `enter_escape`, `enter_ground_attack`, `enter_knee_bend`, `enter_shield`, `enter_squat`, `enter_turn`, `enter_walk` |
| `walk.rs::enter_walk` | Character-aware call: `change_motion_state_at` |
| `walk.rs::walk_input` | Character-aware call: `apply_ground_transition`, `change_motion_state`, `enter_walk` |


Trait and shell adapters declared in `fighter/mod.rs`:

| Function | Why generic |
|---|---|
| `CharacterCallbacks::special_rows` | Returns character-typed MotionRow function pointers |
| `CharacterCallbacks::enter_special` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::kind` | Character identity / descriptor / archive construction contract |
| `CharacterCallbacks::forward_smash_variant` | Character extension point; default reads the C7 descriptor capability |
| `CharacterCallbacks::catch_variant` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::throw_variant` | Character extension point; default reads the C7 descriptor capability |
| `CharacterCallbacks::check_hurtbox_interaction` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::jab_variant` | Character extension point; default reads the C7 descriptor capability |
| `CharacterCallbacks::descriptor` | Character identity / descriptor / archive construction contract |
| `CharacterCallbacks::from_archive` | Character identity / descriptor / archive construction contract |
| `CharacterCallbacks::restore_saved` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::on_load` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::on_reset` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::on_costume_loaded` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::on_resources_loaded` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::on_grounded_motion` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::dynamics_first_force_bone` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::air_dodge_tether` | Character extension point; default reads the C7 descriptor capability |
| `CharacterCallbacks::on_landing` | Character extension point; default reads the C7 descriptor capability |
| `CharacterCallbacks::guard_variant` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::escape_variant` | Character extension point; default reads the C7 descriptor capability |
| `CharacterCallbacks::check_float_input` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::aerial_jump_style` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::multi_jump_attributes` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::multi_jump_family` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::multi_jump_animation` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::aerial_jump_entered` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::aerial_jump_animated` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::action_id` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::animated_shield` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::enter_shield` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::animate_shield` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::input_shield` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::enter_guard_hold` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::enter_guard_off` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::escape_finished` | Character extension point; each implementation owns its behavior/data |
| `CharacterCallbacks::escape_animated` | Character extension point; each implementation owns its behavior/data |
| `Fighter::deref`, `Fighter::deref_mut` | Thin core-reference compatibility adapters |
| `Fighter::install_motion_row` | Converts the typed row to concrete metadata and retains its typed callbacks |
| `MotionRow::clone`, `MotionRow::fmt` | Copy/debug adapters for character-typed function-pointer data |

## Complete melee-ft symbol census

<details><summary>Per-target lines and copies for every melee-ft-owned symbol</summary>

| Target | Symbol | Before lines / copies | After lines / copies |
|---|---|---:|---:|
| sim-lib | `<melee_ft::anim::attach::AnimationPart as core::clone::Clone>::clone` | 2 / 1 | 0 / 0 |
| sim-lib | `<melee_ft::anim::attach::MotionRemap as core::clone::Clone>::clone` | 49 / 1 | 0 / 0 |
| sim-lib | `<melee_ft::anim::playback::Motion as core::clone::Clone>::clone` | 59 / 1 | 0 / 0 |
| sim-lib | `<melee_ft::collision::pose::UnsupportedGroundPose as core::fmt::Debug>::fmt` | 22 / 1 | 0 / 0 |
| sim-lib | `<melee_ft::desc::animation::AnimationDescError as core::error::Error>::source` | 25 / 1 | 25 / 1 |
| sim-lib | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` | 54 / 1 | 54 / 1 |
| sim-lib | `<melee_ft::desc::bones::AnimationBoneSet as core::clone::Clone>::clone` | 10 / 1 | 0 / 0 |
| sim-lib | `<melee_ft::desc::bones::FighterBones as core::clone::Clone>::clone` | 115 / 1 | 0 / 0 |
| sim-lib | `<melee_ft::desc::bones::PartTable as core::clone::Clone>::clone` | 33 / 1 | 0 / 0 |
| sim-lib | `<melee_ft::desc::read::FighterDescError as core::error::Error>::source` | 31 / 1 | 31 / 1 |
| sim-lib | `<melee_ft::desc::read::FighterDescError as core::fmt::Debug>::fmt` | 40 / 1 | 40 / 1 |
| sim-lib | `<melee_ft::fighter::AerialJumpStyle as core::fmt::Debug>::fmt` | 37 / 1 | 37 / 1 |
| sim-lib | `<melee_ft::fighter::caches::DynamicCollider as core::clone::Clone>::clone` | 2 / 1 | 0 / 0 |
| sim-lib | `<melee_ft::fighter::caches::HurtHeight as core::fmt::Debug>::fmt` | 25 / 1 | 0 / 0 |
| sim-lib | `<melee_ft::fighter::caches::Hurtbox as core::clone::Clone>::clone` | 2 / 1 | 0 / 0 |
| sim-lib | `<melee_ft::fighter::commands::Command as core::clone::Clone>::clone` | 106 / 1 | 0 / 0 |
| sim-lib | `<melee_ft::fighter::commands::CommandState as core::default::Default>::default` | 229 / 1 | 0 / 0 |
| sim-lib | `<melee_ft::fighter::effects::EffectRequest as core::fmt::Debug>::fmt` | 134 / 1 | 134 / 1 |
| sim-lib | `<melee_ft::fighter::effects::ResolvedEffect as core::fmt::Debug>::fmt` | 7 / 1 | 7 / 1 |
| sim-lib | `<melee_ft::fighter::hitbox::HitCapsule as core::clone::Clone>::clone` | 31 / 1 | 0 / 0 |
| sim-lib | `<melee_ft::fighter::state::FighterProc as core::fmt::Debug>::fmt` | 72 / 1 | 72 / 1 |
| sim-lib | `<melee_ft::input::iasa::WaitTransition as core::fmt::Debug>::fmt` | 61 / 1 | 61 / 1 |
| sim-lib | `<melee_ft::input::pad::PadSample as core::cmp::PartialEq>::eq` | 56 / 1 | 56 / 1 |
| sim-lib | `<melee_ft::input::pad::PadSample as core::default::Default>::default` | 14 / 1 | 14 / 1 |
| sim-lib | `melee_ft::anim::blend::blend_pose` | 130 / 1 | 130 / 1 |
| sim-lib | `melee_ft::anim::blend::blend_rotation` | 229 / 1 | 229 / 1 |
| sim-lib | `melee_ft::anim::playback::FighterAnimation::advance_main` | 282 / 1 | 282 / 1 |
| sim-lib | `melee_ft::anim::playback::FighterAnimation::advance_main::{{closure}}` | 23 / 3 | 23 / 3 |
| sim-lib | `melee_ft::anim::playback::FighterAnimation::advance_parts` | 164 / 1 | 164 / 1 |
| sim-lib | `melee_ft::anim::playback::FighterAnimation::apply_fall_pose` | 97 / 1 | 0 / 0 |
| sim-lib | `melee_ft::anim::playback::FighterAnimation::apply_fall_pose::{{closure}}` | 17 / 1 | 0 / 0 |
| sim-lib | `melee_ft::anim::playback::FighterAnimation::apply_guard_pose` | 284 / 1 | 0 / 0 |
| sim-lib | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree` | 493 / 1 | 0 / 0 |
| sim-lib | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree::{{closure}}` | 76 / 4 | 0 / 0 |
| sim-lib | `melee_ft::anim::playback::FighterAnimation::step::{{closure}}` | 2 / 2 | 2 / 2 |
| sim-lib | `melee_ft::anim::playback::FighterAnimation::step_with_hooks` | 308 / 8 | 35 / 1 |
| sim-lib | `melee_ft::anim::playback::FighterAnimation::update_wait_with_restart` | 819 / 7 | 0 / 0 |
| sim-lib | `melee_ft::anim::playback::FighterAnimation::update_wait_with_restart::{{closure}}` | 49 / 7 | 0 / 0 |
| sim-lib | `melee_ft::anim::playback::animate_parts` | 110 / 1 | 110 / 1 |
| sim-lib | `melee_ft::anim::playback::for_each_aobj` | 378 / 2 | 0 / 0 |
| sim-lib | `melee_ft::anim::root_motion::RootMotion::animate` | 15 / 1 | 15 / 1 |
| sim-lib | `melee_ft::anim::root_motion::RootMotion::animate_blend` | 15 / 1 | 15 / 1 |
| sim-lib | `melee_ft::anim::root_motion::RootMotion::evaluate` | 308 / 1 | 308 / 1 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::action_id` | 6 / 6 | 6 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::aerial_jump_animated` | 6 / 6 | 6 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::aerial_jump_entered` | 6 / 6 | 6 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::aerial_jump_style` | 4 / 4 | 4 / 4 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::air_dodge_tether` | 63 / 7 | 91 / 7 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::animate_shield` | 12 / 6 | 12 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::animated_shield` | 6 / 6 | 6 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::catch_variant` | 7 / 7 | 7 / 7 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::check_float_input` | 6 / 6 | 6 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::check_hurtbox_interaction` | 6 / 6 | 6 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::dynamics_first_force_bone` | 6 / 6 | 6 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::enter_guard_hold` | 12 / 6 | 12 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::enter_guard_off` | 12 / 6 | 12 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::enter_shield` | 12 / 6 | 12 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::enter_special` | 7 / 7 | 7 / 7 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::escape_animated` | 6 / 6 | 6 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::escape_finished` | 12 / 6 | 12 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::escape_variant` | 108 / 6 | 114 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::forward_smash_variant` | 63 / 7 | 84 / 7 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::guard_variant` | 6 / 6 | 6 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::input_shield` | 12 / 6 | 12 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::jab_variant` | 63 / 7 | 91 / 7 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::multi_jump_attributes` | 6 / 6 | 6 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::multi_jump_family` | 7 / 7 | 7 / 7 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::on_costume_loaded` | 25 / 5 | 25 / 5 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::on_grounded_motion` | 6 / 6 | 6 / 6 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::on_landing` | 45 / 5 | 65 / 5 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::on_resources_loaded` | 5 / 5 | 5 / 5 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::restore_saved` | 3 / 3 | 3 / 3 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::special_rows` | 5 / 5 | 5 / 5 |
| sim-lib | `melee_ft::fighter::CharacterCallbacks::throw_variant` | 63 / 7 | 91 / 7 |
| sim-lib | `melee_ft::fighter::air_dodge::<impl melee_ft::fighter::Fighter<C>>::air_dodge_animation` | 385 / 7 | 385 / 7 |
| sim-lib | `melee_ft::fighter::air_dodge::<impl melee_ft::fighter::Fighter<C>>::air_dodge_input` | 196 / 7 | 196 / 7 |
| sim-lib | `melee_ft::fighter::air_dodge::<impl melee_ft::fighter::Fighter<C>>::enter_air_dodge` | 952 / 7 | 952 / 7 |
| sim-lib | `melee_ft::fighter::attack::<impl melee_ft::fighter::Fighter<C>>::enter_ground_attack` | 1449 / 7 | 1449 / 7 |
| sim-lib | `melee_ft::fighter::attack::<impl melee_ft::fighter::Fighter<C>>::jab_animation` | 406 / 7 | 406 / 7 |
| sim-lib | `melee_ft::fighter::attack::<impl melee_ft::fighter::Fighter<C>>::jab_input` | 896 / 7 | 896 / 7 |
| sim-lib | `melee_ft::fighter::attack::<impl melee_ft::fighter::Fighter<C>>::jab_physics` | 980 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::attack::<impl melee_ft::fighter::Fighter<C>>::tilt_input` | 413 / 7 | 413 / 7 |
| sim-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::begin_damage_reaction` | 3024 / 7 | 357 / 7 |
| sim-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::begin_damage_reaction::{{closure}}` | 35 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::contact_with_hurtboxes` | 1477 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::contact_with_hurtboxes::{{closure}}` | 63 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_animation` | 763 / 7 | 756 / 7 |
| sim-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_collision` | 1624 / 7 | 1617 / 7 |
| sim-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_input` | 1302 / 7 | 1302 / 7 |
| sim-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_input::{{closure}}` | 70 / 7 | 70 / 7 |
| sim-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::damage_physics` | 560 / 7 | 560 / 7 |
| sim-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::decay_air_knockback` | 665 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::process_damage` | 1519 / 7 | 1519 / 7 |
| sim-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::tick_hitlag` | 616 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::damage::DamageParameters::knockback` | 101 / 1 | 0 / 0 |
| sim-lib | `melee_ft::fighter::damage::detect_hit` | 21511 / 49 | 4851 / 49 |
| sim-lib | `melee_ft::fighter::damage::detect_hit::{{closure}}` | 392 / 49 | 0 / 0 |
| sim-lib | `melee_ft::fighter::damage::record_shield_hit` | 9653 / 49 | 0 / 0 |
| sim-lib | `melee_ft::fighter::damage::record_shield_hit::{{closure}}` | 392 / 49 | 0 / 0 |
| sim-lib | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_animation` | 406 / 7 | 406 / 7 |
| sim-lib | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::dash_input` | 1596 / 7 | 1596 / 7 |
| sim-lib | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::enter_dash` | 812 / 7 | 812 / 7 |
| sim-lib | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::reject_dash_attack` | 210 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::reject_running_actions` | 336 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::reject_running_jump` | 252 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::dash::<impl melee_ft::fighter::Fighter<C>>::try_redash` | 973 / 7 | 973 / 7 |
| sim-lib | `melee_ft::fighter::down::<impl melee_ft::fighter::Fighter<C>>::down_animation` | 1141 / 7 | 1134 / 7 |
| sim-lib | `melee_ft::fighter::down::<impl melee_ft::fighter::Fighter<C>>::down_physics` | 1176 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::down::<impl melee_ft::fighter::Fighter<C>>::enter_down_bound` | 1484 / 7 | 1484 / 7 |
| sim-lib | `melee_ft::fighter::down::<impl melee_ft::fighter::Fighter<C>>::try_tech` | 749 / 7 | 749 / 7 |
| sim-lib | `melee_ft::fighter::dynamic_commands::<impl melee_ft::fighter::Fighter<C>>::apply_dynamic_commands` | 1652 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::dynamic_commands::<impl melee_ft::fighter::Fighter<C>>::apply_dynamic_commands::{{closure}}` | 336 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::dynamic_commands::<impl melee_ft::fighter::Fighter<C>>::apply_dynamic_commands::{{closure}}::{{closure}}` | 42 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::effects::<impl melee_ft::fighter::Fighter<C>>::drain_effects` | 420 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::effects::<impl melee_ft::fighter::Fighter<C>>::drain_immediate_effects` | 882 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::effects::<impl melee_ft::fighter::Fighter<C>>::flush_effects_on_motion_change` | 2674 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::effects::<impl melee_ft::fighter::Fighter<C>>::resolve_graphics_commands` | 2856 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::effects::<impl melee_ft::fighter::FighterCore>::drain_effects` | 0 / 0 | 60 / 1 |
| sim-lib | `melee_ft::fighter::effects::<impl melee_ft::fighter::FighterCore>::drain_immediate_effects` | 0 / 0 | 126 / 1 |
| sim-lib | `melee_ft::fighter::entry::<impl melee_ft::fighter::Fighter<C>>::enter_match` | 805 / 7 | 805 / 7 |
| sim-lib | `melee_ft::fighter::entry::<impl melee_ft::fighter::Fighter<C>>::entry_animation` | 847 / 7 | 840 / 7 |
| sim-lib | `melee_ft::fighter::entry::<impl melee_ft::fighter::Fighter<C>>::entry_physics` | 784 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::entry::EntryState::advance` | 45 / 1 | 45 / 1 |
| sim-lib | `melee_ft::fighter::escape::<impl melee_ft::fighter::Fighter<C>>::enter_escape` | 1253 / 7 | 1253 / 7 |
| sim-lib | `melee_ft::fighter::escape::<impl melee_ft::fighter::Fighter<C>>::escape_animation` | 756 / 7 | 742 / 7 |
| sim-lib | `melee_ft::fighter::escape::<impl melee_ft::fighter::Fighter<C>>::escape_physics` | 1113 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::escape::<impl melee_ft::fighter::Fighter<C>>::roll_input` | 455 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::roll_input` | 0 / 0 | 60 / 1 |
| sim-lib | `melee_ft::fighter::fall::<impl melee_ft::fighter::Fighter<C>>::enter_air_dodge_fall` | 777 / 7 | 777 / 7 |
| sim-lib | `melee_ft::fighter::fall::<impl melee_ft::fighter::Fighter<C>>::fall_animation` | 1099 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::fall::<impl melee_ft::fighter::Fighter<C>>::land_from_special_fall` | 420 / 7 | 420 / 7 |
| sim-lib | `melee_ft::fighter::fall::<impl melee_ft::fighter::Fighter<C>>::special_fall_physics` | 175 / 7 | 175 / 7 |
| sim-lib | `melee_ft::fighter::fall::iasa` | 259 / 7 | 259 / 7 |
| sim-lib | `melee_ft::fighter::fall::iasa_with_jump` | 1008 / 14 | 1008 / 14 |
| sim-lib | `melee_ft::fighter::grab::<impl melee_ft::fighter::Fighter<C>>::capture_collision` | 462 / 7 | 455 / 7 |
| sim-lib | `melee_ft::fighter::grab::<impl melee_ft::fighter::Fighter<C>>::catch_animation` | 406 / 7 | 406 / 7 |
| sim-lib | `melee_ft::fighter::grab::<impl melee_ft::fighter::Fighter<C>>::catch_collision` | 679 / 7 | 679 / 7 |
| sim-lib | `melee_ft::fighter::grab::<impl melee_ft::fighter::Fighter<C>>::catch_physics` | 966 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::grab::<impl melee_ft::fighter::Fighter<C>>::enter_catch` | 448 / 7 | 448 / 7 |
| sim-lib | `melee_ft::fighter::grab::<impl melee_ft::fighter::Fighter<C>>::enter_catch_wait` | 525 / 7 | 525 / 7 |
| sim-lib | `melee_ft::fighter::grab::candidate` | 8526 / 49 | 0 / 0 |
| sim-lib | `melee_ft::fighter::grab::capture_delta` | 4116 / 49 | 0 / 0 |
| sim-lib | `melee_ft::fighter::grab::capture_pair` | 12103 / 49 | 8673 / 49 |
| sim-lib | `melee_ft::fighter::grab::capture_wait` | 392 / 7 | 392 / 7 |
| sim-lib | `melee_ft::fighter::grab_throw::<impl melee_ft::fighter::Fighter<C>>::thrown_accessory` | 714 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::grab_throw::<impl melee_ft::fighter::Fighter<C>>::thrown_animation` | 1169 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::grab_throw::back_throw_requested` | 406 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::grab_throw::enter_back_throw` | 20090 / 49 | 8036 / 49 |
| sim-lib | `melee_ft::fighter::grab_throw::enter_back_throw::{{closure}}` | 147 / 49 | 0 / 0 |
| sim-lib | `melee_ft::fighter::grab_throw::release_back_throw` | 25921 / 49 | 2548 / 49 |
| sim-lib | `melee_ft::fighter::grab_throw::release_back_throw::{{closure}}` | 147 / 49 | 0 / 0 |
| sim-lib | `melee_ft::fighter::grab_throw::update_constraint` | 5243 / 49 | 0 / 0 |
| sim-lib | `melee_ft::fighter::jump::<impl melee_ft::fighter::Fighter<C>>::airborne_physics` | 546 / 7 | 154 / 7 |
| sim-lib | `melee_ft::fighter::jump::<impl melee_ft::fighter::Fighter<C>>::apply_fall_gravity` | 546 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::jump::<impl melee_ft::fighter::Fighter<C>>::enter_aerial_jump` | 1435 / 7 | 1435 / 7 |
| sim-lib | `melee_ft::fighter::jump::<impl melee_ft::fighter::Fighter<C>>::enter_knee_bend` | 413 / 7 | 413 / 7 |
| sim-lib | `melee_ft::fighter::jump::<impl melee_ft::fighter::Fighter<C>>::jump_animation` | 826 / 7 | 819 / 7 |
| sim-lib | `melee_ft::fighter::jump::<impl melee_ft::fighter::Fighter<C>>::knee_bend_animation` | 1463 / 7 | 1463 / 7 |
| sim-lib | `melee_ft::fighter::jump::<impl melee_ft::fighter::Fighter<C>>::knee_bend_input` | 595 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::jump::<impl melee_ft::fighter::FighterCore>::apply_fall_gravity` | 0 / 0 | 78 / 1 |
| sim-lib | `melee_ft::fighter::landing::<impl melee_ft::fighter::Fighter<C>>::enter_landing` | 469 / 7 | 469 / 7 |
| sim-lib | `melee_ft::fighter::landing::<impl melee_ft::fighter::Fighter<C>>::enter_landing_squat` | 609 / 7 | 609 / 7 |
| sim-lib | `melee_ft::fighter::landing::<impl melee_ft::fighter::Fighter<C>>::enter_special_landing` | 616 / 7 | 616 / 7 |
| sim-lib | `melee_ft::fighter::landing::<impl melee_ft::fighter::Fighter<C>>::land` | 392 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::landing::<impl melee_ft::fighter::Fighter<C>>::landing_animation` | 406 / 7 | 406 / 7 |
| sim-lib | `melee_ft::fighter::landing::<impl melee_ft::fighter::Fighter<C>>::retained_drop_timer` | 511 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::cliff_climb_animation` | 490 / 7 | 490 / 7 |
| sim-lib | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::cliff_climb_physics` | 2009 / 7 | 798 / 7 |
| sim-lib | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_catch` | 1764 / 7 | 1764 / 7 |
| sim-lib | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::enter_cliff_option` | 546 / 7 | 546 / 7 |
| sim-lib | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation` | 2135 / 7 | 2114 / 7 |
| sim-lib | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision` | 1827 / 7 | 1820 / 7 |
| sim-lib | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision::{{closure}}` | 21 / 7 | 21 / 7 |
| sim-lib | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input` | 2422 / 7 | 2422 / 7 |
| sim-lib | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_jump_physics` | 175 / 7 | 175 / 7 |
| sim-lib | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_physics` | 630 / 7 | 301 / 7 |
| sim-lib | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_position` | 70 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::try_grab_ledge` | 483 / 7 | 483 / 7 |
| sim-lib | `melee_ft::fighter::life::<impl melee_ft::fighter::Fighter<C>>::check_blast_zone` | 1337 / 7 | 1337 / 7 |
| sim-lib | `melee_ft::fighter::life::<impl melee_ft::fighter::Fighter<C>>::death_animation` | 413 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::life::<impl melee_ft::fighter::Fighter<C>>::enter_revival` | 1421 / 7 | 1421 / 7 |
| sim-lib | `melee_ft::fighter::life::<impl melee_ft::fighter::Fighter<C>>::reset_for_revival` | 1843 / 7 | 1843 / 7 |
| sim-lib | `melee_ft::fighter::life::<impl melee_ft::fighter::Fighter<C>>::revival_animation` | 1246 / 7 | 1246 / 7 |
| sim-lib | `melee_ft::fighter::life::<impl melee_ft::fighter::Fighter<C>>::revival_physics` | 700 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::life::<impl melee_ft::fighter::Fighter<C>>::update_revival_platform` | 308 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::aerial_jump_requested` | 728 / 7 | 728 / 7 |
| sim-lib | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::enter_multi_jump` | 1652 / 7 | 1652 / 7 |
| sim-lib | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::multi_jump_animation` | 75 / 1 | 75 / 1 |
| sim-lib | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::multi_jump_physics` | 65 / 1 | 20 / 1 |
| sim-lib | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::Fighter<C>>::multi_jump_turn` | 630 / 7 | 322 / 7 |
| sim-lib | `melee_ft::fighter::pass::<impl melee_ft::fighter::Fighter<C>>::enter_pass` | 924 / 7 | 924 / 7 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::proc_anim` | 945 / 7 | 91 / 7 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::proc_camera` | 126 / 7 | 126 / 7 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::proc_camera_with_map` | 203 / 7 | 203 / 7 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::proc_dynamics_with_map` | 518 / 7 | 98 / 7 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::proc_dynamics_with_map::{{closure}}` | 196 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::proc_hitbox_positions` | 518 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::proc_input` | 924 / 7 | 70 / 7 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::proc_map_with_assets` | 1673 / 7 | 392 / 7 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::proc_pose` | 553 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::proc_process_hit` | 329 / 7 | 133 / 7 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::proc_status` | 609 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::proc_update` | 777 / 7 | 126 / 7 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::resume_wait_animation` | 1274 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::resume_wait_animation::{{closure}}` | 28 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::solve_dynamics` | 1197 / 7 | 539 / 7 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::solve_dynamics::{{closure}}` | 553 / 14 | 0 / 0 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::step_animation` | 1869 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::update_idle_animation` | 924 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::update_idle_animation::{{closure}}` | 455 / 14 | 0 / 0 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::Fighter<C>>::update_idle_animation::{{closure}}::{{closure}}` | 56 / 14 | 0 / 0 |
| sim-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::invalidate_collision_positions` | 0 / 0 | 69 / 1 |
| sim-lib | `melee_ft::fighter::run::<impl melee_ft::fighter::Fighter<C>>::enter_run` | 462 / 7 | 462 / 7 |
| sim-lib | `melee_ft::fighter::run::<impl melee_ft::fighter::Fighter<C>>::enter_run_brake` | 525 / 7 | 525 / 7 |
| sim-lib | `melee_ft::fighter::run::<impl melee_ft::fighter::Fighter<C>>::run_animation` | 462 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::run::<impl melee_ft::fighter::Fighter<C>>::run_brake_animation` | 1008 / 7 | 1008 / 7 |
| sim-lib | `melee_ft::fighter::run::<impl melee_ft::fighter::Fighter<C>>::run_brake_input` | 847 / 7 | 847 / 7 |
| sim-lib | `melee_ft::fighter::run::<impl melee_ft::fighter::Fighter<C>>::run_input` | 931 / 7 | 931 / 7 |
| sim-lib | `melee_ft::fighter::run::<impl melee_ft::fighter::Fighter<C>>::running_physics` | 1071 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::drain_shield` | 630 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_guard_hold` | 763 / 7 | 651 / 7 |
| sim-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_guard_off` | 238 / 7 | 238 / 7 |
| sim-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield` | 2562 / 7 | 2450 / 7 |
| sim-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::guard` | 98 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_animation` | 1358 / 7 | 1351 / 7 |
| sim-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_contact` | 777 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input` | 2317 / 7 | 2310 / 7 |
| sim-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_proc` | 1113 / 7 | 532 / 7 |
| sim-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::take_shield_hit` | 1883 / 7 | 476 / 7 |
| sim-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::update_guard_pose` | 686 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::update_reflect_windows` | 511 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::update_shield_size` | 756 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::update_shield_tilt` | 693 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::smash::<impl melee_ft::fighter::Fighter<C>>::advance_smash_charge` | 490 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::smash::<impl melee_ft::fighter::Fighter<C>>::enter_forward_smash` | 903 / 7 | 903 / 7 |
| sim-lib | `melee_ft::fighter::smash::<impl melee_ft::fighter::Fighter<C>>::update_smash_charge_input` | 567 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::snapshot::<impl melee_types::snapshot::Snapshot for melee_ft::fighter::Fighter<C>>::snapshot` | 490 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::spawn::<impl melee_ft::fighter::CpuState>::initialize` | 95 / 1 | 95 / 1 |
| sim-lib | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::change_motion_state_with_options` | 5117 / 7 | 504 / 7 |
| sim-lib | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::change_motion_state_with_options::{{closure}}` | 14 / 14 | 0 / 0 |
| sim-lib | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::create` | 3577 / 7 | 3630 / 7 |
| sim-lib | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::create::{{closure}}` | 21 / 7 | 21 / 7 |
| sim-lib | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::prepare` | 7375 / 7 | 970 / 7 |
| sim-lib | `melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::prepare::{{closure}}` | 280 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::enter_squat` | 483 / 7 | 483 / 7 |
| sim-lib | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_animation` | 497 / 7 | 490 / 7 |
| sim-lib | `melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input` | 2121 / 7 | 2107 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::capture` | 56 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::catch` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::catch_pull` | 413 / 7 | 413 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::cliff_catch` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::cliff_climb` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::damage` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::dash` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::dead` | 49 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::down_bound` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::entry` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::escape` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::escape_air` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::fall` | 336 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::guard_on` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::jab` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::knee_bend` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::landing` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::multi_jump` | 48 / 1 | 48 / 1 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::pass` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::revival` | 609 / 7 | 609 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::run` | 63 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::run_brake` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::squat` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::squat_wait` | 28 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::tech_roll` | 392 / 7 | 392 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::throw` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::thrown` | 63 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::turn` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::turn_run` | 336 / 7 | 336 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::wait` | 28 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::animation::walk` | 63 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::camera::cliff` | 14 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::camera::follow_fighter` | 14 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::camera::update` | 966 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::capture` | 434 / 7 | 434 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::catch` | 434 / 7 | 434 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::cliff_catch` | 434 / 7 | 434 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::cliff_climb` | 434 / 7 | 434 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::damage` | 434 / 7 | 434 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::entry` | 665 / 7 | 42 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::escape` | 350 / 7 | 350 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::escape_air` | 560 / 7 | 560 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::fall` | 434 / 7 | 434 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::fall_collision` | 1505 / 7 | 1505 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::fall_special` | 434 / 7 | 434 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::finish_ground` | 1064 / 7 | 1064 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::ground_action` | 350 / 7 | 350 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::ground_wait` | 350 / 7 | 350 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::guard_set_off` | 427 / 7 | 427 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::jump` | 434 / 7 | 434 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::pass` | 434 / 7 | 434 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::revival` | 749 / 7 | 42 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::revival::{{closure}}` | 42 / 14 | 0 / 0 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::running` | 350 / 7 | 350 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::thrown` | 112 / 7 | 42 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::collision::turn_run` | 434 / 7 | 434 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::aerial` | 448 / 7 | 448 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::aerial::{{closure}}` | 308 / 7 | 308 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::catch` | 7 / 7 | 7 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::cliff_catch` | 7 / 7 | 7 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::cliff_climb` | 7 / 7 | 7 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::cliff_wait` | 35 / 7 | 35 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::damage` | 231 / 7 | 231 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::dash` | 231 / 7 | 231 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::entry` | 7 / 7 | 7 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::escape` | 147 / 7 | 140 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::escape_air` | 14 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::escape_n` | 7 / 7 | 7 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::fall_special` | 119 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::guard_on` | 231 / 7 | 231 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::jab` | 231 / 7 | 231 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::knee_bend` | 210 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::landing` | 469 / 7 | 469 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::run` | 231 / 7 | 231 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::run_brake` | 35 / 7 | 35 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::squat` | 231 / 7 | 231 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::tilt` | 231 / 7 | 231 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::turn` | 231 / 7 | 231 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::turn_run` | 14 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::wait` | 252 / 7 | 252 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::input::walk` | 231 / 7 | 231 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::capture` | 84 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::catch` | 56 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::cliff_catch` | 133 / 7 | 133 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::cliff_climb` | 392 / 7 | 392 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::cliff_jump2` | 98 / 7 | 98 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::damage` | 56 / 7 | 56 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::dead` | 7 / 7 | 7 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::down` | 56 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::entry` | 133 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::escape` | 273 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::escape_air` | 287 / 7 | 287 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::fall` | 294 / 7 | 294 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::fall_special` | 98 / 7 | 98 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::guard_on` | 343 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::jab` | 56 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::multi_jump` | 15 / 1 | 15 / 1 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::pass` | 126 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::revival` | 42 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::running` | 651 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::turn_run` | 651 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::wait` | 266 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::callbacks::physics::walk` | 819 / 7 | 14 / 7 |
| sim-lib | `melee_ft::fighter::state::row::<impl melee_ft::fighter::Fighter<C>>::row` | 805 / 7 | 805 / 7 |
| sim-lib | `melee_ft::fighter::state::row::unimplemented_anim` | 35 / 7 | 35 / 7 |
| sim-lib | `melee_ft::fighter::state::row::unimplemented_camera` | 35 / 7 | 35 / 7 |
| sim-lib | `melee_ft::fighter::state::row::unimplemented_collision` | 35 / 7 | 35 / 7 |
| sim-lib | `melee_ft::fighter::state::row::unimplemented_iasa` | 35 / 7 | 35 / 7 |
| sim-lib | `melee_ft::fighter::state::row::unimplemented_physics` | 35 / 7 | 35 / 7 |
| sim-lib | `melee_ft::fighter::state::special::<impl melee_ft::fighter::Fighter<C>>::enter_buffered_special` | 959 / 7 | 959 / 7 |
| sim-lib | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::enter_turn` | 721 / 7 | 721 / 7 |
| sim-lib | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_animation` | 756 / 7 | 756 / 7 |
| sim-lib | `melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input` | 2219 / 7 | 2191 / 7 |
| sim-lib | `melee_ft::fighter::turn_run::<impl melee_ft::fighter::Fighter<C>>::enter_turn_run` | 182 / 7 | 182 / 7 |
| sim-lib | `melee_ft::fighter::turn_run::<impl melee_ft::fighter::Fighter<C>>::try_turn_run` | 406 / 7 | 406 / 7 |
| sim-lib | `melee_ft::fighter::turn_run::<impl melee_ft::fighter::Fighter<C>>::turn_run_animation` | 1260 / 7 | 1260 / 7 |
| sim-lib | `melee_ft::fighter::turn_run::<impl melee_ft::fighter::Fighter<C>>::turn_run_collision` | 714 / 7 | 714 / 7 |
| sim-lib | `melee_ft::fighter::turn_run::<impl melee_ft::fighter::Fighter<C>>::turn_run_physics` | 1183 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::apply_ground_transition` | 945 / 7 | 945 / 7 |
| sim-lib | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::enter_walk` | 693 / 7 | 693 / 7 |
| sim-lib | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::first_ground_transition` | 357 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::first_ground_transition::{{closure}}` | 105 / 14 | 0 / 0 |
| sim-lib | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_animation` | 539 / 7 | 0 / 0 |
| sim-lib | `melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input` | 2163 / 7 | 2142 / 7 |
| sim-lib | `melee_ft::physics::airborne::drift_acceleration` | 108 / 1 | 0 / 0 |
| sim-lib | `melee_ft::physics::grounded::accelerate_toward` | 122 / 1 | 0 / 0 |
| sim-lib | `melee_ft::physics::integrate::integrate_environment` | 62 / 1 | 62 / 1 |
| ft-lib | `<melee_ft::anim::attach::AnimationPart as core::clone::Clone>::clone` | 0 / 0 | 2 / 1 |
| ft-lib | `<melee_ft::anim::attach::MotionRemap as core::clone::Clone>::clone` | 0 / 0 | 49 / 1 |
| ft-lib | `<melee_ft::anim::playback::Motion as core::clone::Clone>::clone` | 0 / 0 | 59 / 1 |
| ft-lib | `<melee_ft::collision::pose::UnsupportedGroundPose as core::fmt::Debug>::fmt` | 0 / 0 | 22 / 1 |
| ft-lib | `<melee_ft::desc::animation::AnimationDescError as core::error::Error>::source` | 25 / 1 | 25 / 1 |
| ft-lib | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Debug>::fmt` | 54 / 1 | 54 / 1 |
| ft-lib | `<melee_ft::desc::animation::AnimationDescError as core::fmt::Display>::fmt` | 413 / 1 | 413 / 1 |
| ft-lib | `<melee_ft::desc::attributes::AirAttributes as melee_types::snapshot::Snapshot>::snapshot` | 16 / 1 | 16 / 1 |
| ft-lib | `<melee_ft::desc::attributes::CameraAttributes as melee_types::snapshot::Snapshot>::snapshot` | 6 / 1 | 6 / 1 |
| ft-lib | `<melee_ft::desc::attributes::CombatAttributes as melee_types::snapshot::Snapshot>::snapshot` | 16 / 1 | 16 / 1 |
| ft-lib | `<melee_ft::desc::attributes::FighterAttributes as melee_types::snapshot::Snapshot>::snapshot` | 34 / 1 | 34 / 1 |
| ft-lib | `<melee_ft::desc::attributes::GroundAttributes as melee_types::snapshot::Snapshot>::snapshot` | 6 / 1 | 6 / 1 |
| ft-lib | `<melee_ft::desc::attributes::IceAttributes as melee_types::snapshot::Snapshot>::snapshot` | 10 / 1 | 10 / 1 |
| ft-lib | `<melee_ft::desc::attributes::ItemsAttributes as melee_types::snapshot::Snapshot>::snapshot` | 18 / 1 | 18 / 1 |
| ft-lib | `<melee_ft::desc::attributes::JumpingAttributes as melee_types::snapshot::Snapshot>::snapshot` | 18 / 1 | 18 / 1 |
| ft-lib | `<melee_ft::desc::attributes::KirbyThrowAttributes as melee_types::snapshot::Snapshot>::snapshot` | 2 / 1 | 2 / 1 |
| ft-lib | `<melee_ft::desc::attributes::LandingAttributes as melee_types::snapshot::Snapshot>::snapshot` | 12 / 1 | 12 / 1 |
| ft-lib | `<melee_ft::desc::attributes::LedgeAttributes as melee_types::snapshot::Snapshot>::snapshot` | 4 / 1 | 4 / 1 |
| ft-lib | `<melee_ft::desc::attributes::RunningAttributes as melee_types::snapshot::Snapshot>::snapshot` | 12 / 1 | 12 / 1 |
| ft-lib | `<melee_ft::desc::attributes::ShieldAttributes as melee_types::snapshot::Snapshot>::snapshot` | 4 / 1 | 4 / 1 |
| ft-lib | `<melee_ft::desc::attributes::SizeAttributes as melee_types::snapshot::Snapshot>::snapshot` | 14 / 1 | 14 / 1 |
| ft-lib | `<melee_ft::desc::attributes::SpecialsAttributes as melee_types::snapshot::Snapshot>::snapshot` | 2 / 1 | 2 / 1 |
| ft-lib | `<melee_ft::desc::attributes::WalkingAttributes as melee_types::snapshot::Snapshot>::snapshot` | 12 / 1 | 12 / 1 |
| ft-lib | `<melee_ft::desc::attributes::WallAttributes as melee_types::snapshot::Snapshot>::snapshot` | 10 / 1 | 10 / 1 |
| ft-lib | `<melee_ft::desc::attributes::YoshiEggAttributes as melee_types::snapshot::Snapshot>::snapshot` | 8 / 1 | 8 / 1 |
| ft-lib | `<melee_ft::desc::bones::AnimationBoneSet as core::clone::Clone>::clone` | 0 / 0 | 10 / 1 |
| ft-lib | `<melee_ft::desc::bones::FighterBones as core::clone::Clone>::clone` | 0 / 0 | 115 / 1 |
| ft-lib | `<melee_ft::desc::bones::PartTable as core::clone::Clone>::clone` | 0 / 0 | 33 / 1 |
| ft-lib | `<melee_ft::desc::read::FighterDescError as core::convert::From<hsd_archive::error::Error>>::from` | 6 / 1 | 6 / 1 |
| ft-lib | `<melee_ft::desc::read::FighterDescError as core::error::Error>::source` | 31 / 1 | 31 / 1 |
| ft-lib | `<melee_ft::desc::read::FighterDescError as core::fmt::Debug>::fmt` | 40 / 1 | 40 / 1 |
| ft-lib | `<melee_ft::desc::read::FighterDescError as core::fmt::Display>::fmt` | 159 / 1 | 159 / 1 |
| ft-lib | `<melee_ft::fighter::RetailTrig as hsd_anim::mtx::InverseTrig>::acosf` | 2 / 1 | 2 / 1 |
| ft-lib | `<melee_ft::fighter::RetailTrig as hsd_anim::mtx::InverseTrig>::asinf` | 2 / 1 | 2 / 1 |
| ft-lib | `<melee_ft::fighter::RetailTrig as hsd_anim::mtx::InverseTrig>::atan2f` | 2 / 1 | 2 / 1 |
| ft-lib | `<melee_ft::fighter::caches::DynamicCollider as core::clone::Clone>::clone` | 0 / 0 | 2 / 1 |
| ft-lib | `<melee_ft::fighter::caches::HurtHeight as core::fmt::Debug>::fmt` | 0 / 0 | 25 / 1 |
| ft-lib | `<melee_ft::fighter::caches::Hurtbox as core::clone::Clone>::clone` | 0 / 0 | 2 / 1 |
| ft-lib | `<melee_ft::fighter::caches::ThrownHitbox as core::default::Default>::default` | 18 / 1 | 18 / 1 |
| ft-lib | `<melee_ft::fighter::commands::Command as core::clone::Clone>::clone` | 106 / 1 | 106 / 1 |
| ft-lib | `<melee_ft::fighter::commands::CommandState as core::default::Default>::default` | 0 / 0 | 229 / 1 |
| ft-lib | `<melee_ft::fighter::hitbox::HitCapsule as core::clone::Clone>::clone` | 31 / 1 | 31 / 1 |
| ft-lib | `<melee_ft::fighter::state::action::ActionId as core::convert::From<melee_types::motion_state::CommonMotionState>>::from` | 23 / 1 | 23 / 1 |
| ft-lib | `<melee_ft::input::iasa::WaitTransition as core::fmt::Debug>::fmt` | 0 / 0 | 61 / 1 |
| ft-lib | `melee_ft::anim::attach::attach_motion` | 878 / 1 | 878 / 1 |
| ft-lib | `melee_ft::anim::attach::attach_motion::{{closure}}` | 101 / 3 | 101 / 3 |
| ft-lib | `melee_ft::anim::blend::blend_pose` | 0 / 0 | 130 / 1 |
| ft-lib | `melee_ft::anim::blend::blend_rotation` | 0 / 0 | 229 / 1 |
| ft-lib | `melee_ft::anim::blend::copy_pose` | 43 / 1 | 43 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::advance_main` | 0 / 0 | 282 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::advance_main::{{closure}}` | 0 / 0 | 23 / 3 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::advance_parts` | 0 / 0 | 164 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::apply_fall_pose` | 0 / 0 | 97 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::apply_fall_pose::{{closure}}` | 0 / 0 | 17 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::apply_guard_pose` | 0 / 0 | 284 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::clear_motion` | 24 / 1 | 24 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::current_aobj` | 55 / 1 | 55 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::current_aobj::{{closure}}` | 45 / 2 | 45 / 2 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::frames_remaining` | 43 / 1 | 43 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::frames_remaining::{{closure}}` | 56 / 2 | 56 / 2 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::new` | 258 / 1 | 258 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::new::{{closure}}` | 85 / 2 | 85 / 2 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::reset_pose` | 7 / 1 | 7 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::reset_pose_range` | 250 / 1 | 250 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree` | 0 / 0 | 493 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::resume_dynamic_subtree::{{closure}}` | 0 / 0 | 76 / 4 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::set_animation` | 178 / 1 | 178 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::set_animation::{{closure}}::{{closure}}` | 19 / 1 | 19 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::set_rate` | 19 / 1 | 19 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::set_rate::{{closure}}` | 10 / 2 | 10 / 2 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::set_secondary_animation` | 111 / 1 | 111 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::set_secondary_animation::{{closure}}` | 19 / 1 | 19 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::step_with_hooks` | 0 / 0 | 39 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::update_wait_with_restart` | 0 / 0 | 117 / 1 |
| ft-lib | `melee_ft::anim::playback::FighterAnimation::update_wait_with_restart::{{closure}}` | 0 / 0 | 7 / 1 |
| ft-lib | `melee_ft::anim::playback::animate_parts` | 0 / 0 | 110 / 1 |
| ft-lib | `melee_ft::anim::playback::first_aobj` | 28 / 1 | 28 / 1 |
| ft-lib | `melee_ft::anim::playback::first_aobj::{{closure}}` | 27 / 1 | 27 / 1 |
| ft-lib | `melee_ft::anim::playback::for_each_aobj` | 752 / 4 | 1130 / 6 |
| ft-lib | `melee_ft::anim::root_motion::RootMotion::animate` | 0 / 0 | 15 / 1 |
| ft-lib | `melee_ft::anim::root_motion::RootMotion::animate_blend` | 0 / 0 | 15 / 1 |
| ft-lib | `melee_ft::anim::root_motion::RootMotion::compensate` | 53 / 1 | 53 / 1 |
| ft-lib | `melee_ft::anim::root_motion::RootMotion::evaluate` | 0 / 0 | 308 / 1 |
| ft-lib | `melee_ft::anim::wait_choice::choose_wait_animation` | 82 / 1 | 82 / 1 |
| ft-lib | `melee_ft::anim::wait_choice::choose_wait_animation::{{closure}}` | 36 / 2 | 36 / 2 |
| ft-lib | `melee_ft::collision::air::begin_map` | 27 / 1 | 27 / 1 |
| ft-lib | `melee_ft::collision::air::collide_air_dodge` | 57 / 1 | 57 / 1 |
| ft-lib | `melee_ft::collision::air::collide_air_dodge::{{closure}}` | 3 / 1 | 3 / 1 |
| ft-lib | `melee_ft::collision::air::collide_entry` | 50 / 1 | 50 / 1 |
| ft-lib | `melee_ft::collision::air::collide_fall` | 98 / 1 | 98 / 1 |
| ft-lib | `melee_ft::collision::air::collide_fall::{{closure}}` | 10 / 4 | 10 / 4 |
| ft-lib | `melee_ft::collision::air::collide_pass` | 90 / 1 | 90 / 1 |
| ft-lib | `melee_ft::collision::air::collide_pass::{{closure}}` | 6 / 2 | 6 / 2 |
| ft-lib | `melee_ft::collision::ecb::EcbPose::position` | 59 / 1 | 59 / 1 |
| ft-lib | `melee_ft::collision::ecb::EcbPose::position::{{closure}}` | 6 / 1 | 6 / 1 |
| ft-lib | `melee_ft::collision::ecb::EcbPose::read` | 232 / 1 | 232 / 1 |
| ft-lib | `melee_ft::collision::ecb::EcbPose::read::{{closure}}` | 5 / 1 | 5 / 1 |
| ft-lib | `melee_ft::collision::ecb::initialize` | 258 / 1 | 258 / 1 |
| ft-lib | `melee_ft::collision::ecb::initialize::{{closure}}` | 22 / 1 | 22 / 1 |
| ft-lib | `melee_ft::collision::ecb::load_grounded` | 34 / 1 | 34 / 1 |
| ft-lib | `melee_ft::collision::ecb::load_grounded::{{closure}}` | 3 / 1 | 3 / 1 |
| ft-lib | `melee_ft::collision::ecb::world_position` | 49 / 1 | 49 / 1 |
| ft-lib | `melee_ft::collision::ground::collide_wait` | 105 / 1 | 105 / 1 |
| ft-lib | `melee_ft::collision::ground::collide_wait::{{closure}}` | 6 / 2 | 6 / 2 |
| ft-lib | `melee_ft::collision::ground::map_escape` | 65 / 1 | 65 / 1 |
| ft-lib | `melee_ft::collision::ground::map_escape::{{closure}}` | 3 / 1 | 3 / 1 |
| ft-lib | `melee_ft::collision::ground::map_ground_action` | 65 / 1 | 65 / 1 |
| ft-lib | `melee_ft::collision::ground::map_ground_action::{{closure}}` | 3 / 1 | 3 / 1 |
| ft-lib | `melee_ft::collision::ground::map_wait` | 55 / 1 | 55 / 1 |
| ft-lib | `melee_ft::collision::ground::resume_wait` | 52 / 1 | 52 / 1 |
| ft-lib | `melee_ft::collision::ground::resume_wait::{{closure}}` | 3 / 1 | 3 / 1 |
| ft-lib | `melee_ft::collision::pose::FlatGroundPose::update` | 350 / 1 | 350 / 1 |
| ft-lib | `melee_ft::collision::pose::FlatGroundPose::update::{{closure}}` | 29 / 2 | 29 / 2 |
| ft-lib | `melee_ft::collision::pose::align_flat_foot` | 155 / 1 | 155 / 1 |
| ft-lib | `melee_ft::collision::pose::normalize` | 26 / 1 | 26 / 1 |
| ft-lib | `melee_ft::desc::animation::AnimationEntry::sub_archive` | 81 / 1 | 81 / 1 |
| ft-lib | `melee_ft::desc::animation::read_entry` | 550 / 1 | 550 / 1 |
| ft-lib | `melee_ft::desc::animation::read_entry::{{closure}}` | 72 / 1 | 72 / 1 |
| ft-lib | `melee_ft::desc::animation::read_fighter_animations` | 389 / 1 | 389 / 1 |
| ft-lib | `melee_ft::desc::animation::read_fighter_animations::{{closure}}` | 53 / 1 | 53 / 1 |
| ft-lib | `melee_ft::desc::animation::read_named_fighter_animations` | 63 / 1 | 63 / 1 |
| ft-lib | `melee_ft::desc::animation::read_named_fighter_animations::{{closure}}` | 52 / 1 | 52 / 1 |
| ft-lib | `melee_ft::desc::animation::read_pointer` | 133 / 1 | 133 / 1 |
| ft-lib | `melee_ft::desc::attributes::AirAttributes::read` | 427 / 1 | 427 / 1 |
| ft-lib | `melee_ft::desc::attributes::CameraAttributes::read` | 165 / 1 | 165 / 1 |
| ft-lib | `melee_ft::desc::attributes::CombatAttributes::read` | 427 / 1 | 427 / 1 |
| ft-lib | `melee_ft::desc::attributes::FighterAttributes::read` | 885 / 1 | 885 / 1 |
| ft-lib | `melee_ft::desc::attributes::GroundAttributes::read` | 167 / 1 | 167 / 1 |
| ft-lib | `melee_ft::desc::attributes::IceAttributes::read` | 271 / 1 | 271 / 1 |
| ft-lib | `melee_ft::desc::attributes::ItemsAttributes::read` | 491 / 1 | 491 / 1 |
| ft-lib | `melee_ft::desc::attributes::JumpingAttributes::read` | 479 / 1 | 479 / 1 |
| ft-lib | `melee_ft::desc::attributes::KirbyThrowAttributes::read` | 58 / 1 | 58 / 1 |
| ft-lib | `melee_ft::desc::attributes::LandingAttributes::read` | 323 / 1 | 323 / 1 |
| ft-lib | `melee_ft::desc::attributes::LedgeAttributes::read` | 109 / 1 | 109 / 1 |
| ft-lib | `melee_ft::desc::attributes::RunningAttributes::read` | 323 / 1 | 323 / 1 |
| ft-lib | `melee_ft::desc::attributes::ShieldAttributes::read` | 109 / 1 | 109 / 1 |
| ft-lib | `melee_ft::desc::attributes::SizeAttributes::read` | 375 / 1 | 375 / 1 |
| ft-lib | `melee_ft::desc::attributes::SpecialsAttributes::read` | 58 / 1 | 58 / 1 |
| ft-lib | `melee_ft::desc::attributes::WalkingAttributes::read` | 323 / 1 | 323 / 1 |
| ft-lib | `melee_ft::desc::attributes::WallAttributes::read` | 271 / 1 | 271 / 1 |
| ft-lib | `melee_ft::desc::attributes::YoshiEggAttributes::read` | 215 / 1 | 215 / 1 |
| ft-lib | `melee_ft::desc::attributes::read_fighter_attributes` | 44 / 1 | 44 / 1 |
| ft-lib | `melee_ft::desc::bones::EcbBones::read` | 603 / 1 | 603 / 1 |
| ft-lib | `melee_ft::desc::bones::PartTable::read` | 423 / 1 | 423 / 1 |
| ft-lib | `melee_ft::desc::bones::PartTable::read::{{closure}}` | 54 / 3 | 54 / 3 |
| ft-lib | `melee_ft::desc::bones::byte_array` | 237 / 1 | 237 / 1 |
| ft-lib | `melee_ft::desc::bones::optional_bone` | 96 / 1 | 96 / 1 |
| ft-lib | `melee_ft::desc::bones::optional_bone::{{closure}}` | 67 / 1 | 67 / 1 |
| ft-lib | `melee_ft::desc::bones::read_animation_sets` | 507 / 1 | 507 / 1 |
| ft-lib | `melee_ft::desc::bones::read_animation_sets::{{closure}}` | 2 / 1 | 2 / 1 |
| ft-lib | `melee_ft::desc::bones::read_bone_array` | 358 / 1 | 358 / 1 |
| ft-lib | `melee_ft::desc::bones::read_bone_array::{{closure}}` | 59 / 1 | 59 / 1 |
| ft-lib | `melee_ft::desc::bones::read_dynamics_bones` | 216 / 1 | 216 / 1 |
| ft-lib | `melee_ft::desc::bones::read_fighter_bones` | 1042 / 1 | 1042 / 1 |
| ft-lib | `melee_ft::desc::bones::read_ground_pose` | 640 / 1 | 640 / 1 |
| ft-lib | `melee_ft::desc::bones::read_part_table` | 145 / 1 | 145 / 1 |
| ft-lib | `melee_ft::desc::common::CommonFighterData::read` | 1374 / 1 | 1374 / 1 |
| ft-lib | `melee_ft::desc::common::read_common_data` | 84 / 1 | 84 / 1 |
| ft-lib | `melee_ft::desc::fox_attributes::BlasterAttributes::read` | 577 / 1 | 577 / 1 |
| ft-lib | `melee_ft::desc::fox_attributes::FireFoxAttributes::read` | 929 / 1 | 929 / 1 |
| ft-lib | `melee_ft::desc::fox_attributes::FoxAttributes::read` | 281 / 1 | 281 / 1 |
| ft-lib | `melee_ft::desc::fox_attributes::IllusionAttributes::read` | 659 / 1 | 659 / 1 |
| ft-lib | `melee_ft::desc::fox_attributes::ReflectionAttributes::read` | 490 / 1 | 490 / 1 |
| ft-lib | `melee_ft::desc::fox_attributes::ReflectorAttributes::read` | 387 / 1 | 387 / 1 |
| ft-lib | `melee_ft::desc::playback::read_playback_motion` | 1101 / 1 | 1101 / 1 |
| ft-lib | `melee_ft::desc::playback::read_squat_table` | 374 / 1 | 374 / 1 |
| ft-lib | `melee_ft::desc::playback::read_wait_table` | 374 / 1 | 374 / 1 |
| ft-lib | `melee_ft::desc::read::block` | 75 / 1 | 75 / 1 |
| ft-lib | `melee_ft::desc::read::pointer` | 198 / 1 | 198 / 1 |
| ft-lib | `melee_ft::desc::read::public` | 29 / 1 | 29 / 1 |
| ft-lib | `melee_ft::desc::read::public::{{closure}}` | 56 / 1 | 56 / 1 |
| ft-lib | `melee_ft::desc::read::required` | 80 / 1 | 80 / 1 |
| ft-lib | `melee_ft::desc::read::special_attributes_offset` | 2 / 1 | 2 / 1 |
| ft-lib | `melee_ft::desc::read::vec3` | 268 / 1 | 268 / 1 |
| ft-lib | `melee_ft::dynamics::read_motion_starts` | 844 / 1 | 844 / 1 |
| ft-lib | `melee_ft::dynamics::read_motion_starts::{{closure}}` | 20 / 1 | 20 / 1 |
| ft-lib | `melee_ft::dynamics::read_sets` | 1231 / 1 | 1231 / 1 |
| ft-lib | `melee_ft::dynamics::read_sets::{{closure}}` | 136 / 1 | 136 / 1 |
| ft-lib | `melee_ft::dynamics::select` | 166 / 1 | 166 / 1 |
| ft-lib | `melee_ft::dynamics::select::{{closure}}` | 6 / 1 | 6 / 1 |
| ft-lib | `melee_ft::fighter::Status::require_supported` | 52 / 1 | 52 / 1 |
| ft-lib | `melee_ft::fighter::air_dodge::AirDodgeParameters::read` | 376 / 1 | 376 / 1 |
| ft-lib | `melee_ft::fighter::assets::FighterAssets::load` | 21431 / 1 | 21431 / 1 |
| ft-lib | `melee_ft::fighter::assets::FighterAssets::load::{{closure}}` | 292 / 12 | 292 / 12 |
| ft-lib | `melee_ft::fighter::assets::flatten_part` | 165 / 1 | 165 / 1 |
| ft-lib | `melee_ft::fighter::assets::flatten_part::{{closure}}` | 50 / 1 | 50 / 1 |
| ft-lib | `melee_ft::fighter::assets::flatten_part::{{closure}}::{{closure}}` | 26 / 1 | 26 / 1 |
| ft-lib | `melee_ft::fighter::assets::read_dynamic_colliders` | 307 / 1 | 307 / 1 |
| ft-lib | `melee_ft::fighter::assets::read_dynamic_colliders::{{closure}}` | 195 / 1 | 195 / 1 |
| ft-lib | `melee_ft::fighter::assets::read_graphics` | 527 / 1 | 527 / 1 |
| ft-lib | `melee_ft::fighter::assets::read_guard_pose` | 454 / 1 | 454 / 1 |
| ft-lib | `melee_ft::fighter::assets::read_guard_pose::{{closure}}` | 14 / 1 | 14 / 1 |
| ft-lib | `melee_ft::fighter::assets::read_hurtboxes` | 294 / 1 | 294 / 1 |
| ft-lib | `melee_ft::fighter::assets::read_hurtboxes::{{closure}}` | 399 / 1 | 399 / 1 |
| ft-lib | `melee_ft::fighter::assets::read_script` | 1389 / 1 | 1389 / 1 |
| ft-lib | `melee_ft::fighter::assets::read_vec` | 171 / 1 | 171 / 1 |
| ft-lib | `melee_ft::fighter::attack::<impl melee_ft::fighter::FighterCore>::jab_physics` | 0 / 0 | 140 / 1 |
| ft-lib | `melee_ft::fighter::caches::HurtHeight::from_retail` | 32 / 1 | 32 / 1 |
| ft-lib | `melee_ft::fighter::caches::ThrownHitbox::update` | 68 / 1 | 68 / 1 |
| ft-lib | `melee_ft::fighter::caches::bone_position` | 49 / 1 | 49 / 1 |
| ft-lib | `melee_ft::fighter::caches::hurtbox_extents` | 212 / 1 | 212 / 1 |
| ft-lib | `melee_ft::fighter::caches::hurtbox_extents::{{closure}}` | 9 / 1 | 9 / 1 |
| ft-lib | `melee_ft::fighter::commands::CommandState::seek` | 2 / 1 | 2 / 1 |
| ft-lib | `melee_ft::fighter::commands::CommandState::step` | 2 / 1 | 2 / 1 |
| ft-lib | `melee_ft::fighter::commands::CommandState::step_inner` | 923 / 1 | 923 / 1 |
| ft-lib | `melee_ft::fighter::commands::CommandState::step_inner::{{closure}}` | 6 / 1 | 6 / 1 |
| ft-lib | `melee_ft::fighter::commands::apply_part` | 233 / 1 | 233 / 1 |
| ft-lib | `melee_ft::fighter::commands::apply_part::{{closure}}` | 3 / 1 | 3 / 1 |
| ft-lib | `melee_ft::fighter::commands::reset_parts` | 141 / 1 | 141 / 1 |
| ft-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::FighterCore>::contact_with_hurtboxes` | 0 / 0 | 211 / 1 |
| ft-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::FighterCore>::contact_with_hurtboxes::{{closure}}` | 0 / 0 | 9 / 1 |
| ft-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::FighterCore>::decay_air_knockback` | 0 / 0 | 95 / 1 |
| ft-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::FighterCore>::finish_damage_reaction` | 0 / 0 | 52 / 1 |
| ft-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::FighterCore>::prepare_damage_reaction` | 0 / 0 | 341 / 1 |
| ft-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::FighterCore>::prepare_damage_reaction::{{closure}}` | 0 / 0 | 5 / 1 |
| ft-lib | `melee_ft::fighter::damage::<impl melee_ft::fighter::FighterCore>::tick_hitlag` | 0 / 0 | 88 / 1 |
| ft-lib | `melee_ft::fighter::damage::DamageParameters::knockback` | 0 / 0 | 101 / 1 |
| ft-lib | `melee_ft::fighter::damage::DamageParameters::read` | 1723 / 1 | 1723 / 1 |
| ft-lib | `melee_ft::fighter::damage::detect_eligible_hit` | 0 / 0 | 366 / 1 |
| ft-lib | `melee_ft::fighter::damage::detect_eligible_hit::{{closure}}` | 0 / 0 | 8 / 1 |
| ft-lib | `melee_ft::fighter::damage::record_shield_hit` | 0 / 0 | 197 / 1 |
| ft-lib | `melee_ft::fighter::damage::record_shield_hit::{{closure}}` | 0 / 0 | 8 / 1 |
| ft-lib | `melee_ft::fighter::dash::<impl melee_ft::fighter::FighterCore>::reject_dash_attack` | 0 / 0 | 30 / 1 |
| ft-lib | `melee_ft::fighter::dash::<impl melee_ft::fighter::FighterCore>::reject_running_actions` | 0 / 0 | 48 / 1 |
| ft-lib | `melee_ft::fighter::dash::<impl melee_ft::fighter::FighterCore>::reject_running_jump` | 0 / 0 | 36 / 1 |
| ft-lib | `melee_ft::fighter::down::<impl melee_ft::fighter::FighterCore>::down_physics` | 0 / 0 | 168 / 1 |
| ft-lib | `melee_ft::fighter::dynamic_commands::<impl melee_ft::fighter::FighterCore>::apply_dynamic_commands` | 0 / 0 | 236 / 1 |
| ft-lib | `melee_ft::fighter::dynamic_commands::<impl melee_ft::fighter::FighterCore>::apply_dynamic_commands::{{closure}}` | 0 / 0 | 48 / 1 |
| ft-lib | `melee_ft::fighter::dynamic_commands::<impl melee_ft::fighter::FighterCore>::apply_dynamic_commands::{{closure}}::{{closure}}` | 0 / 0 | 6 / 1 |
| ft-lib | `melee_ft::fighter::effects::<impl melee_ft::fighter::FighterCore>::flush_effects_on_motion_change` | 0 / 0 | 382 / 1 |
| ft-lib | `melee_ft::fighter::effects::<impl melee_ft::fighter::FighterCore>::resolve_graphics_commands` | 0 / 0 | 408 / 1 |
| ft-lib | `melee_ft::fighter::entry::<impl melee_ft::fighter::FighterCore>::entry_physics` | 0 / 0 | 109 / 1 |
| ft-lib | `melee_ft::fighter::escape::<impl melee_ft::fighter::FighterCore>::escape_physics` | 0 / 0 | 159 / 1 |
| ft-lib | `melee_ft::fighter::fall::<impl melee_ft::fighter::FighterCore>::fall_animation` | 0 / 0 | 157 / 1 |
| ft-lib | `melee_ft::fighter::fall::FallState::advance` | 37 / 1 | 37 / 1 |
| ft-lib | `melee_ft::fighter::grab::<impl melee_ft::fighter::FighterCore>::catch_physics` | 0 / 0 | 138 / 1 |
| ft-lib | `melee_ft::fighter::grab::align_capture` | 0 / 0 | 35 / 1 |
| ft-lib | `melee_ft::fighter::grab::candidate` | 0 / 0 | 173 / 1 |
| ft-lib | `melee_ft::fighter::grab::capture_delta` | 0 / 0 | 84 / 1 |
| ft-lib | `melee_ft::fighter::grab::finish_capture` | 0 / 0 | 71 / 1 |
| ft-lib | `melee_ft::fighter::grab_throw::<impl melee_ft::fighter::FighterCore>::thrown_accessory` | 0 / 0 | 102 / 1 |
| ft-lib | `melee_ft::fighter::grab_throw::<impl melee_ft::fighter::FighterCore>::thrown_animation` | 0 / 0 | 167 / 1 |
| ft-lib | `melee_ft::fighter::grab_throw::back_throw_requested` | 0 / 0 | 57 / 1 |
| ft-lib | `melee_ft::fighter::grab_throw::finish_thrown_pose` | 0 / 0 | 91 / 1 |
| ft-lib | `melee_ft::fighter::grab_throw::prepare_throw_release` | 0 / 0 | 474 / 1 |
| ft-lib | `melee_ft::fighter::grab_throw::prepare_throw_release::{{closure}}` | 0 / 0 | 3 / 1 |
| ft-lib | `melee_ft::fighter::grab_throw::prepare_thrown_pose` | 0 / 0 | 210 / 1 |
| ft-lib | `melee_ft::fighter::grab_throw::prepare_thrown_pose::{{closure}}` | 0 / 0 | 3 / 1 |
| ft-lib | `melee_ft::fighter::grab_throw::update_constraint` | 0 / 0 | 107 / 1 |
| ft-lib | `melee_ft::fighter::hitbox::HitCapsule::update` | 61 / 1 | 61 / 1 |
| ft-lib | `melee_ft::fighter::hitbox::HitboxDescriptor::read` | 514 / 1 | 514 / 1 |
| ft-lib | `melee_ft::fighter::hitbox::ThrowHitbox::read` | 244 / 1 | 244 / 1 |
| ft-lib | `melee_ft::fighter::hitbox::spawn` | 135 / 1 | 135 / 1 |
| ft-lib | `melee_ft::fighter::hitbox::spawn::{{closure}}` | 11 / 2 | 11 / 2 |
| ft-lib | `melee_ft::fighter::jump::<impl melee_ft::fighter::FighterCore>::airborne_physics` | 0 / 0 | 68 / 1 |
| ft-lib | `melee_ft::fighter::jump::<impl melee_ft::fighter::FighterCore>::apply_fall_gravity` | 0 / 0 | 78 / 1 |
| ft-lib | `melee_ft::fighter::jump::<impl melee_ft::fighter::FighterCore>::knee_bend_input` | 0 / 0 | 85 / 1 |
| ft-lib | `melee_ft::fighter::landing::<impl melee_ft::fighter::FighterCore>::land` | 0 / 0 | 56 / 1 |
| ft-lib | `melee_ft::fighter::landing::<impl melee_ft::fighter::FighterCore>::retained_drop_timer` | 0 / 0 | 73 / 1 |
| ft-lib | `melee_ft::fighter::landing::iasa` | 138 / 1 | 138 / 1 |
| ft-lib | `melee_ft::fighter::ledge::<impl melee_ft::fighter::FighterCore>::cliff_ground_physics` | 0 / 0 | 175 / 1 |
| ft-lib | `melee_ft::fighter::ledge::<impl melee_ft::fighter::FighterCore>::ledge_position` | 0 / 0 | 10 / 1 |
| ft-lib | `melee_ft::fighter::ledge::<impl melee_ft::fighter::FighterCore>::place_at_ledge` | 0 / 0 | 49 / 1 |
| ft-lib | `melee_ft::fighter::ledge::LedgeParameters::read` | 438 / 1 | 438 / 1 |
| ft-lib | `melee_ft::fighter::life::<impl melee_ft::fighter::FighterCore>::death_animation` | 0 / 0 | 59 / 1 |
| ft-lib | `melee_ft::fighter::life::<impl melee_ft::fighter::FighterCore>::revival_physics` | 0 / 0 | 100 / 1 |
| ft-lib | `melee_ft::fighter::life::<impl melee_ft::fighter::FighterCore>::update_revival_platform` | 0 / 0 | 44 / 1 |
| ft-lib | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::FighterCore>::multi_jump_drift` | 0 / 0 | 47 / 1 |
| ft-lib | `melee_ft::fighter::multi_jump::<impl melee_ft::fighter::FighterCore>::turn_multi_jump_model` | 0 / 0 | 54 / 1 |
| ft-lib | `melee_ft::fighter::multi_jump::MultiJumpAttributes::contains_action` | 19 / 1 | 19 / 1 |
| ft-lib | `melee_ft::fighter::multi_jump::MultiJumpAttributes::contains_action::{{closure}}` | 25 / 1 | 25 / 1 |
| ft-lib | `melee_ft::fighter::overlap::nudge` | 290 / 1 | 290 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::begin_animation_phase` | 0 / 0 | 128 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::begin_physics_phase` | 0 / 0 | 36 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::dynamics_frame` | 0 / 0 | 54 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::dynamics_frame::{{closure}}` | 0 / 0 | 11 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::proc_accessories` | 0 / 0 | 9 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::proc_cpu_gate` | 0 / 0 | 25 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::proc_grab` | 0 / 0 | 9 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::proc_hit_detection` | 0 / 0 | 9 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::proc_hitbox_positions` | 0 / 0 | 74 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::proc_pose` | 0 / 0 | 79 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::proc_status` | 0 / 0 | 87 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::resolve_landing_effects` | 0 / 0 | 186 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::resume_wait_animation` | 0 / 0 | 181 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::resume_wait_animation::{{closure}}` | 0 / 0 | 4 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::sample_input` | 0 / 0 | 133 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::solve_dynamic_set` | 0 / 0 | 64 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::solve_dynamic_set::{{closure}}` | 0 / 0 | 102 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::step_animation` | 0 / 0 | 267 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::update_dynamic_colliders` | 0 / 0 | 60 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::update_hurtbox_extents` | 0 / 0 | 30 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::update_idle_animation` | 0 / 0 | 131 / 1 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::update_idle_animation::{{closure}}` | 0 / 0 | 65 / 2 |
| ft-lib | `melee_ft::fighter::procs::<impl melee_ft::fighter::FighterCore>::update_idle_animation::{{closure}}::{{closure}}` | 0 / 0 | 8 / 2 |
| ft-lib | `melee_ft::fighter::run::<impl melee_ft::fighter::FighterCore>::run_animation` | 0 / 0 | 66 / 1 |
| ft-lib | `melee_ft::fighter::run::<impl melee_ft::fighter::FighterCore>::running_physics` | 0 / 0 | 153 / 1 |
| ft-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::FighterCore>::apply_shield_impact` | 0 / 0 | 192 / 1 |
| ft-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::FighterCore>::drain_shield` | 0 / 0 | 90 / 1 |
| ft-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::FighterCore>::guard` | 0 / 0 | 14 / 1 |
| ft-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::FighterCore>::queue_shield_effect` | 0 / 0 | 18 / 1 |
| ft-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::FighterCore>::shield_contact` | 0 / 0 | 111 / 1 |
| ft-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::FighterCore>::update_guard_pose` | 0 / 0 | 98 / 1 |
| ft-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::FighterCore>::update_reflect_windows` | 0 / 0 | 73 / 1 |
| ft-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::FighterCore>::update_shield_health` | 0 / 0 | 85 / 1 |
| ft-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::FighterCore>::update_shield_size` | 0 / 0 | 108 / 1 |
| ft-lib | `melee_ft::fighter::shield::<impl melee_ft::fighter::FighterCore>::update_shield_tilt` | 0 / 0 | 99 / 1 |
| ft-lib | `melee_ft::fighter::shield::ShieldParameters::read` | 1682 / 1 | 1682 / 1 |
| ft-lib | `melee_ft::fighter::shield::ShieldState::reflect_hit` | 5 / 1 | 5 / 1 |
| ft-lib | `melee_ft::fighter::smash::<impl melee_ft::fighter::FighterCore>::advance_smash_charge` | 0 / 0 | 70 / 1 |
| ft-lib | `melee_ft::fighter::smash::<impl melee_ft::fighter::FighterCore>::update_smash_charge_input` | 0 / 0 | 81 / 1 |
| ft-lib | `melee_ft::fighter::snapshot::<impl melee_types::snapshot::Snapshot for melee_ft::fighter::FighterCore>::snapshot` | 0 / 0 | 70 / 1 |
| ft-lib | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::begin_motion_change` | 0 / 0 | 84 / 1 |
| ft-lib | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::clear_animation` | 0 / 0 | 9 / 1 |
| ft-lib | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare` | 0 / 0 | 969 / 1 |
| ft-lib | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::prepare::{{closure}}` | 0 / 0 | 40 / 1 |
| ft-lib | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::reset_motion` | 0 / 0 | 328 / 1 |
| ft-lib | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::start_motion_animation` | 0 / 0 | 254 / 1 |
| ft-lib | `melee_ft::fighter::spawn::<impl melee_ft::fighter::FighterCore>::start_motion_animation::{{closure}}` | 0 / 0 | 2 / 2 |
| ft-lib | `melee_ft::fighter::state::callbacks::animation::<impl melee_ft::fighter::FighterCore>::animation_capture` | 0 / 0 | 8 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::animation::<impl melee_ft::fighter::FighterCore>::animation_dead` | 0 / 0 | 7 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::animation::<impl melee_ft::fighter::FighterCore>::animation_fall` | 0 / 0 | 48 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::animation::<impl melee_ft::fighter::FighterCore>::animation_run` | 0 / 0 | 9 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::animation::<impl melee_ft::fighter::FighterCore>::animation_squat_wait` | 0 / 0 | 4 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::animation::<impl melee_ft::fighter::FighterCore>::animation_thrown` | 0 / 0 | 9 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::animation::<impl melee_ft::fighter::FighterCore>::animation_wait` | 0 / 0 | 4 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::animation::<impl melee_ft::fighter::FighterCore>::animation_walk` | 0 / 0 | 9 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::camera::update` | 0 / 0 | 138 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::collision::<impl melee_ft::fighter::FighterCore>::collision_entry` | 0 / 0 | 94 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::collision::<impl melee_ft::fighter::FighterCore>::collision_revival` | 0 / 0 | 106 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::collision::<impl melee_ft::fighter::FighterCore>::collision_revival::{{closure}}` | 0 / 0 | 6 / 2 |
| ft-lib | `melee_ft::fighter::state::callbacks::collision::<impl melee_ft::fighter::FighterCore>::collision_thrown` | 0 / 0 | 16 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::input::<impl melee_ft::fighter::FighterCore>::input_fall_special` | 0 / 0 | 17 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::input::<impl melee_ft::fighter::FighterCore>::input_knee_bend` | 0 / 0 | 30 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::input::<impl melee_ft::fighter::FighterCore>::input_turn_run` | 0 / 0 | 2 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::physics::<impl melee_ft::fighter::FighterCore>::physics_capture` | 0 / 0 | 12 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::physics::<impl melee_ft::fighter::FighterCore>::physics_catch` | 0 / 0 | 8 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::physics::<impl melee_ft::fighter::FighterCore>::physics_down` | 0 / 0 | 8 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::physics::<impl melee_ft::fighter::FighterCore>::physics_entry` | 0 / 0 | 19 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::physics::<impl melee_ft::fighter::FighterCore>::physics_escape` | 0 / 0 | 39 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::physics::<impl melee_ft::fighter::FighterCore>::physics_guard_on` | 0 / 0 | 49 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::physics::<impl melee_ft::fighter::FighterCore>::physics_jab` | 0 / 0 | 8 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::physics::<impl melee_ft::fighter::FighterCore>::physics_pass` | 0 / 0 | 18 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::physics::<impl melee_ft::fighter::FighterCore>::physics_revival` | 0 / 0 | 6 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::physics::<impl melee_ft::fighter::FighterCore>::physics_running` | 0 / 0 | 93 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::physics::<impl melee_ft::fighter::FighterCore>::physics_turn_run` | 0 / 0 | 93 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::physics::<impl melee_ft::fighter::FighterCore>::physics_wait` | 0 / 0 | 38 / 1 |
| ft-lib | `melee_ft::fighter::state::callbacks::physics::<impl melee_ft::fighter::FighterCore>::physics_walk` | 0 / 0 | 117 / 1 |
| ft-lib | `melee_ft::fighter::state::row::unsupported_action` | 68 / 1 | 68 / 1 |
| ft-lib | `melee_ft::fighter::turn_run::<impl melee_ft::fighter::FighterCore>::turn_run_physics` | 0 / 0 | 169 / 1 |
| ft-lib | `melee_ft::fighter::walk::<impl melee_ft::fighter::FighterCore>::first_ground_transition` | 0 / 0 | 51 / 1 |
| ft-lib | `melee_ft::fighter::walk::<impl melee_ft::fighter::FighterCore>::first_ground_transition::{{closure}}` | 0 / 0 | 15 / 2 |
| ft-lib | `melee_ft::fighter::walk::<impl melee_ft::fighter::FighterCore>::walk_animation` | 0 / 0 | 76 / 1 |
| ft-lib | `melee_ft::input::common::InputCommonData::read` | 1123 / 1 | 1123 / 1 |
| ft-lib | `melee_ft::input::geometry::crosses_stick_circle` | 105 / 1 | 105 / 1 |
| ft-lib | `melee_ft::input::human::count_joystick_activity` | 80 / 1 | 80 / 1 |
| ft-lib | `melee_ft::input::human::increment_analog` | 5 / 1 | 5 / 1 |
| ft-lib | `melee_ft::input::human::run_cpu_input_proc` | 12 / 1 | 12 / 1 |
| ft-lib | `melee_ft::input::human::sample_input` | 204 / 1 | 204 / 1 |
| ft-lib | `melee_ft::input::human::update_action_timers` | 189 / 1 | 189 / 1 |
| ft-lib | `melee_ft::input::human::update_analog_timers` | 116 / 1 | 116 / 1 |
| ft-lib | `melee_ft::input::human::update_axis` | 46 / 1 | 46 / 1 |
| ft-lib | `melee_ft::input::human::update_button_timers` | 148 / 1 | 148 / 1 |
| ft-lib | `melee_ft::input::human::update_human_input` | 150 / 1 | 150 / 1 |
| ft-lib | `melee_ft::input::human::update_input` | 41 / 1 | 41 / 1 |
| ft-lib | `melee_ft::input::iasa::attack_matches` | 206 / 1 | 206 / 1 |
| ft-lib | `melee_ft::input::iasa::evaluate` | 287 / 1 | 287 / 1 |
| ft-lib | `melee_ft::input::iasa::wait_iasa` | 2 / 1 | 2 / 1 |
| ft-lib | `melee_ft::input::iasa::wait_iasa::{{closure}}` | 1 / 1 | 1 / 1 |
| ft-lib | `melee_ft::input::iasa::wait_iasa_observe` | 127 / 1 | 127 / 1 |
| ft-lib | `melee_ft::input::pad::PadSample::from_origin_adjusted` | 56 / 1 | 56 / 1 |
| ft-lib | `melee_ft::input::pad::normalize_stick` | 120 / 1 | 120 / 1 |
| ft-lib | `melee_ft::physics::FighterPhysics::standing` | 65 / 1 | 65 / 1 |
| ft-lib | `melee_ft::physics::airborne::drift` | 22 / 1 | 22 / 1 |
| ft-lib | `melee_ft::physics::airborne::drift_acceleration` | 108 / 1 | 108 / 1 |
| ft-lib | `melee_ft::physics::airborne::fall_physics` | 37 / 1 | 37 / 1 |
| ft-lib | `melee_ft::physics::grounded::accelerate_toward` | 122 / 1 | 122 / 1 |
| ft-lib | `melee_ft::physics::grounded::finish_ground_update` | 31 / 1 | 31 / 1 |
| ft-lib | `melee_ft::physics::grounded::floor_friction` | 15 / 1 | 15 / 1 |
| ft-lib | `melee_ft::physics::grounded::friction_physics` | 118 / 1 | 118 / 1 |
| ft-lib | `melee_ft::physics::grounded::ground_knockback` | 58 / 1 | 58 / 1 |
| ft-lib | `melee_ft::physics::grounded::step_wait` | 8 / 1 | 8 / 1 |
| ft-lib | `melee_ft::physics::grounded::wait_physics` | 4 / 1 | 4 / 1 |
| ft-lib | `melee_ft::physics::grounded::walk_physics` | 59 / 1 | 59 / 1 |
| ft-lib | `melee_ft::physics::integrate::VelocityBlend::apply` | 60 / 1 | 60 / 1 |
| ft-lib | `melee_ft::physics::integrate::integrate_environment` | 62 / 1 | 62 / 1 |
| ft-lib | `melee_ft::physics::integrate::integrate_velocity` | 145 / 1 | 145 / 1 |

</details>

## First 60 llvm-lines output rows

<details><summary>Before: sim-bin</summary>

```text
  Lines                Copies              Function name
  -----                ------              -------------
  52267                1647                (TOTAL)
   1309 (2.5%,  2.5%)     1 (0.1%,  0.1%)  <melee_sim::Command as clap_builder::derive::Subcommand>::augment_subcommands
   1306 (2.5%,  5.0%)     2 (0.1%,  0.2%)  melee_diff::_::<impl serde_core::ser::Serialize for melee_diff::Value>::serialize
   1260 (2.4%,  7.4%)    20 (1.2%,  1.4%)  <serde_json::ser::Compound<W,F> as serde_core::ser::SerializeMap>::serialize_value
    967 (1.9%,  9.3%)     1 (0.1%,  1.5%)  <melee_sim::Command as clap_builder::derive::FromArgMatches>::from_arg_matches_mut
    960 (1.8%, 11.1%)    16 (1.0%,  2.4%)  <alloc::sync::Weak<T,A> as core::ops::drop::Drop>::drop
    856 (1.6%, 12.7%)     8 (0.5%,  2.9%)  clap_builder::parser::matches::arg_matches::ArgMatches::try_remove_arg_t
    802 (1.5%, 14.3%)     1 (0.1%,  3.0%)  hsd_anim::jobj::JObjTree::update_func
    764 (1.5%, 15.7%)     8 (0.5%,  3.5%)  <alloc::vec::Vec<T> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<T,I>>::from_iter
    668 (1.3%, 17.0%)    20 (1.2%,  4.7%)  <alloc::boxed::Box<T,A> as core::ops::drop::Drop>::drop
    668 (1.3%, 18.3%)     7 (0.4%,  5.1%)  clap_builder::parser::matches::arg_matches::ArgMatches::try_remove_one
    639 (1.2%, 19.5%)     1 (0.1%,  5.2%)  melee_sim::main
    595 (1.1%, 20.7%)    26 (1.6%,  6.7%)  alloc::boxed::Box<T>::new
    566 (1.1%, 21.7%)     8 (0.5%,  7.2%)  clap_builder::util::any_value::AnyValue::downcast_into
    556 (1.1%, 22.8%)     4 (0.2%,  7.5%)  alloc::collections::btree::navigate::<impl alloc::collections::btree::node::Handle<alloc::collections::btree::node::NodeRef<BorrowType,K,V,alloc::collections::btree::node::marker::LeafOrInternal>,alloc::collections::btree::node::marker::KV>>::next_leaf_edge
    545 (1.0%, 23.8%)     1 (0.1%,  7.5%)  <i64>::from_ascii_radix
    539 (1.0%, 24.9%)     8 (0.5%,  8.0%)  alloc::sync::Arc<T,A>::try_unwrap
    528 (1.0%, 25.9%)     8 (0.5%,  8.5%)  alloc::sync::Arc<dyn core::any::Any+core::marker::Sync+core::marker::Send,A>::downcast
    503 (1.0%, 26.8%)     1 (0.1%,  8.6%)  clap_builder::builder::value_parser::RangedU64ValueParser<T>::format_bounds
    485 (0.9%, 27.8%)     6 (0.4%,  8.9%)  <F as clap_builder::builder::value_parser::TypedValueParser>::parse_ref::{{closure}}
    484 (0.9%, 28.7%)     4 (0.2%,  9.2%)  <melee_diff::_::<impl serde_core::ser::Serialize for melee_diff::Value>::serialize::__AdjacentlyTagged as serde_core::ser::Serialize>::serialize
    449 (0.9%, 29.6%)     4 (0.2%,  9.4%)  alloc::collections::btree::node::NodeRef<BorrowType,K,V,Type>::ascend
    444 (0.8%, 30.4%)     1 (0.1%,  9.5%)  melee_sim::bones::write_fox_wait1_bones
    443 (0.8%, 31.3%)     8 (0.5%, 10.0%)  clap_builder::parser::error::MatchesError::unwrap
    428 (0.8%, 32.1%)     4 (0.2%, 10.2%)  alloc::collections::btree::navigate::LazyLeafRange<BorrowType,K,V>::init_front
    410 (0.8%, 32.9%)     1 (0.1%, 10.3%)  clap_builder::builder::command::Command::try_get_matches_from_mut
    406 (0.8%, 33.6%)     7 (0.4%, 10.7%)  clap_builder::builder::arg::Arg::value_parser
    401 (0.8%, 34.4%)     5 (0.3%, 11.0%)  clap_builder::builder::arg_group::ArgGroup::args
    386 (0.7%, 35.1%)     5 (0.3%, 11.3%)  core::iter::traits::iterator::Iterator::try_fold
    373 (0.7%, 35.9%)     4 (0.2%, 11.5%)  alloc::vec::Vec<T,A>::extend_desugared
    367 (0.7%, 36.6%)     5 (0.3%, 11.8%)  <clap_builder::builder::value_parser::RangedI64ValueParser<T> as clap_builder::builder::value_parser::TypedValueParser>::parse_ref::{{closure}}
    367 (0.7%, 37.3%)     5 (0.3%, 12.1%)  <clap_builder::builder::value_parser::RangedU64ValueParser<T> as clap_builder::builder::value_parser::TypedValueParser>::parse_ref::{{closure}}
    359 (0.7%, 37.9%)    10 (0.6%, 12.8%)  core::result::Result<T,E>::expect
    359 (0.7%, 38.6%)     1 (0.1%, 12.8%)  <clap_builder::builder::value_parser::RangedI64ValueParser<T> as clap_builder::builder::value_parser::TypedValueParser>::parse_ref
    357 (0.7%, 39.3%)     3 (0.2%, 13.0%)  alloc::collections::btree::navigate::<impl alloc::collections::btree::node::Handle<alloc::collections::btree::node::NodeRef<alloc::collections::btree::node::marker::Dying,K,V,alloc::collections::btree::node::marker::Leaf>,alloc::collections::btree::node::marker::Edge>>::deallocating_next
    357 (0.7%, 40.0%)     1 (0.1%, 13.1%)  <clap_builder::builder::value_parser::RangedU64ValueParser<T> as clap_builder::builder::value_parser::TypedValueParser>::parse_ref
    350 (0.7%, 40.7%)     3 (0.2%, 13.2%)  <T as alloc::slice::<impl [T]>::to_vec_in::ConvertVec>::to_vec
    345 (0.7%, 41.3%)    46 (2.8%, 16.0%)  <alloc::vec::Vec<T,A> as core::ops::drop::Drop>::drop
    340 (0.7%, 42.0%)    20 (1.2%, 17.2%)  serde_core::ser::SerializeMap::serialize_entry
    332 (0.6%, 42.6%)     2 (0.1%, 17.4%)  serde_json::ser::format_escaped_str_contents
    324 (0.6%, 43.2%)     2 (0.1%, 17.5%)  std::io::buffered::bufwriter::BufWriter<W>::flush_buf
    315 (0.6%, 43.8%)     3 (0.2%, 17.7%)  alloc::collections::btree::navigate::LazyLeafRange<alloc::collections::btree::node::marker::Dying,K,V>::take_front
    311 (0.6%, 44.4%)     6 (0.4%, 18.0%)  <P as clap_builder::builder::value_parser::AnyValueParser>::parse_ref
    311 (0.6%, 45.0%)     6 (0.4%, 18.4%)  <P as clap_builder::builder::value_parser::AnyValueParser>::parse_ref_
    306 (0.6%, 45.6%)     3 (0.2%, 18.6%)  <F as clap_builder::builder::value_parser::TypedValueParser>::parse_ref
    300 (0.6%, 46.2%)     4 (0.2%, 18.8%)  alloc::vec::Vec<T,A>::extend_trusted
    297 (0.6%, 46.8%)     1 (0.1%, 18.9%)  melee_sim::trace::write_run
    288 (0.6%, 47.3%)     2 (0.1%, 19.0%)  serde_core::ser::Serializer::collect_map
    284 (0.5%, 47.8%)     2 (0.1%, 19.1%)  melee_diff::_::<impl serde_core::ser::Serialize for melee_diff::Record>::serialize
    282 (0.5%, 48.4%)     2 (0.1%, 19.2%)  clap_builder::error::Error<F>::raw
    277 (0.5%, 48.9%)     1 (0.1%, 19.3%)  clap_builder::error::Error<F>::with_cmd
    276 (0.5%, 49.4%)     4 (0.2%, 19.6%)  <serde_json::ser::Compound<W,F> as serde_core::ser::SerializeMap>::serialize_key
    271 (0.5%, 50.0%)     1 (0.1%, 19.6%)  <u64>::from_ascii_radix
    271 (0.5%, 50.5%)     1 (0.1%, 19.7%)  <usize>::from_ascii_radix
    260 (0.5%, 51.0%)     5 (0.3%, 20.0%)  <core::iter::adapters::enumerate::Enumerate<I> as core::iter::traits::iterator::Iterator>::next
    256 (0.5%, 51.5%)     4 (0.2%, 20.2%)  <core::ops::index_range::IndexRange as core::iter::traits::iterator::Iterator>::try_fold
    252 (0.5%, 52.0%)     3 (0.2%, 20.4%)  <alloc::collections::btree::map::BTreeMap<K,V,A> as core::ops::drop::Drop>::drop
    247 (0.5%, 52.4%)     1 (0.1%, 20.5%)  melee_sim::parse_bone_scale
```

</details>

<details><summary>After: sim-bin</summary>

```text
  Lines                Copies              Function name
  -----                ------              -------------
  52267                1647                (TOTAL)
   1309 (2.5%,  2.5%)     1 (0.1%,  0.1%)  <melee_sim::Command as clap_builder::derive::Subcommand>::augment_subcommands
   1306 (2.5%,  5.0%)     2 (0.1%,  0.2%)  melee_diff::_::<impl serde_core::ser::Serialize for melee_diff::Value>::serialize
   1260 (2.4%,  7.4%)    20 (1.2%,  1.4%)  <serde_json::ser::Compound<W,F> as serde_core::ser::SerializeMap>::serialize_value
    967 (1.9%,  9.3%)     1 (0.1%,  1.5%)  <melee_sim::Command as clap_builder::derive::FromArgMatches>::from_arg_matches_mut
    960 (1.8%, 11.1%)    16 (1.0%,  2.4%)  <alloc::sync::Weak<T,A> as core::ops::drop::Drop>::drop
    856 (1.6%, 12.7%)     8 (0.5%,  2.9%)  clap_builder::parser::matches::arg_matches::ArgMatches::try_remove_arg_t
    802 (1.5%, 14.3%)     1 (0.1%,  3.0%)  hsd_anim::jobj::JObjTree::update_func
    764 (1.5%, 15.7%)     8 (0.5%,  3.5%)  <alloc::vec::Vec<T> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<T,I>>::from_iter
    668 (1.3%, 17.0%)    20 (1.2%,  4.7%)  <alloc::boxed::Box<T,A> as core::ops::drop::Drop>::drop
    668 (1.3%, 18.3%)     7 (0.4%,  5.1%)  clap_builder::parser::matches::arg_matches::ArgMatches::try_remove_one
    639 (1.2%, 19.5%)     1 (0.1%,  5.2%)  melee_sim::main
    595 (1.1%, 20.7%)    26 (1.6%,  6.7%)  alloc::boxed::Box<T>::new
    566 (1.1%, 21.7%)     8 (0.5%,  7.2%)  clap_builder::util::any_value::AnyValue::downcast_into
    556 (1.1%, 22.8%)     4 (0.2%,  7.5%)  alloc::collections::btree::navigate::<impl alloc::collections::btree::node::Handle<alloc::collections::btree::node::NodeRef<BorrowType,K,V,alloc::collections::btree::node::marker::LeafOrInternal>,alloc::collections::btree::node::marker::KV>>::next_leaf_edge
    545 (1.0%, 23.8%)     1 (0.1%,  7.5%)  <i64>::from_ascii_radix
    539 (1.0%, 24.9%)     8 (0.5%,  8.0%)  alloc::sync::Arc<T,A>::try_unwrap
    528 (1.0%, 25.9%)     8 (0.5%,  8.5%)  alloc::sync::Arc<dyn core::any::Any+core::marker::Sync+core::marker::Send,A>::downcast
    503 (1.0%, 26.8%)     1 (0.1%,  8.6%)  clap_builder::builder::value_parser::RangedU64ValueParser<T>::format_bounds
    485 (0.9%, 27.8%)     6 (0.4%,  8.9%)  <F as clap_builder::builder::value_parser::TypedValueParser>::parse_ref::{{closure}}
    484 (0.9%, 28.7%)     4 (0.2%,  9.2%)  <melee_diff::_::<impl serde_core::ser::Serialize for melee_diff::Value>::serialize::__AdjacentlyTagged as serde_core::ser::Serialize>::serialize
    449 (0.9%, 29.6%)     4 (0.2%,  9.4%)  alloc::collections::btree::node::NodeRef<BorrowType,K,V,Type>::ascend
    444 (0.8%, 30.4%)     1 (0.1%,  9.5%)  melee_sim::bones::write_fox_wait1_bones
    443 (0.8%, 31.3%)     8 (0.5%, 10.0%)  clap_builder::parser::error::MatchesError::unwrap
    428 (0.8%, 32.1%)     4 (0.2%, 10.2%)  alloc::collections::btree::navigate::LazyLeafRange<BorrowType,K,V>::init_front
    410 (0.8%, 32.9%)     1 (0.1%, 10.3%)  clap_builder::builder::command::Command::try_get_matches_from_mut
    406 (0.8%, 33.6%)     7 (0.4%, 10.7%)  clap_builder::builder::arg::Arg::value_parser
    401 (0.8%, 34.4%)     5 (0.3%, 11.0%)  clap_builder::builder::arg_group::ArgGroup::args
    386 (0.7%, 35.1%)     5 (0.3%, 11.3%)  core::iter::traits::iterator::Iterator::try_fold
    373 (0.7%, 35.9%)     4 (0.2%, 11.5%)  alloc::vec::Vec<T,A>::extend_desugared
    367 (0.7%, 36.6%)     5 (0.3%, 11.8%)  <clap_builder::builder::value_parser::RangedI64ValueParser<T> as clap_builder::builder::value_parser::TypedValueParser>::parse_ref::{{closure}}
    367 (0.7%, 37.3%)     5 (0.3%, 12.1%)  <clap_builder::builder::value_parser::RangedU64ValueParser<T> as clap_builder::builder::value_parser::TypedValueParser>::parse_ref::{{closure}}
    359 (0.7%, 37.9%)    10 (0.6%, 12.8%)  core::result::Result<T,E>::expect
    359 (0.7%, 38.6%)     1 (0.1%, 12.8%)  <clap_builder::builder::value_parser::RangedI64ValueParser<T> as clap_builder::builder::value_parser::TypedValueParser>::parse_ref
    357 (0.7%, 39.3%)     3 (0.2%, 13.0%)  alloc::collections::btree::navigate::<impl alloc::collections::btree::node::Handle<alloc::collections::btree::node::NodeRef<alloc::collections::btree::node::marker::Dying,K,V,alloc::collections::btree::node::marker::Leaf>,alloc::collections::btree::node::marker::Edge>>::deallocating_next
    357 (0.7%, 40.0%)     1 (0.1%, 13.1%)  <clap_builder::builder::value_parser::RangedU64ValueParser<T> as clap_builder::builder::value_parser::TypedValueParser>::parse_ref
    350 (0.7%, 40.7%)     3 (0.2%, 13.2%)  <T as alloc::slice::<impl [T]>::to_vec_in::ConvertVec>::to_vec
    345 (0.7%, 41.3%)    46 (2.8%, 16.0%)  <alloc::vec::Vec<T,A> as core::ops::drop::Drop>::drop
    340 (0.7%, 42.0%)    20 (1.2%, 17.2%)  serde_core::ser::SerializeMap::serialize_entry
    332 (0.6%, 42.6%)     2 (0.1%, 17.4%)  serde_json::ser::format_escaped_str_contents
    324 (0.6%, 43.2%)     2 (0.1%, 17.5%)  std::io::buffered::bufwriter::BufWriter<W>::flush_buf
    315 (0.6%, 43.8%)     3 (0.2%, 17.7%)  alloc::collections::btree::navigate::LazyLeafRange<alloc::collections::btree::node::marker::Dying,K,V>::take_front
    311 (0.6%, 44.4%)     6 (0.4%, 18.0%)  <P as clap_builder::builder::value_parser::AnyValueParser>::parse_ref
    311 (0.6%, 45.0%)     6 (0.4%, 18.4%)  <P as clap_builder::builder::value_parser::AnyValueParser>::parse_ref_
    306 (0.6%, 45.6%)     3 (0.2%, 18.6%)  <F as clap_builder::builder::value_parser::TypedValueParser>::parse_ref
    300 (0.6%, 46.2%)     4 (0.2%, 18.8%)  alloc::vec::Vec<T,A>::extend_trusted
    297 (0.6%, 46.8%)     1 (0.1%, 18.9%)  melee_sim::trace::write_run
    288 (0.6%, 47.3%)     2 (0.1%, 19.0%)  serde_core::ser::Serializer::collect_map
    284 (0.5%, 47.8%)     2 (0.1%, 19.1%)  melee_diff::_::<impl serde_core::ser::Serialize for melee_diff::Record>::serialize
    282 (0.5%, 48.4%)     2 (0.1%, 19.2%)  clap_builder::error::Error<F>::raw
    277 (0.5%, 48.9%)     1 (0.1%, 19.3%)  clap_builder::error::Error<F>::with_cmd
    276 (0.5%, 49.4%)     4 (0.2%, 19.6%)  <serde_json::ser::Compound<W,F> as serde_core::ser::SerializeMap>::serialize_key
    271 (0.5%, 50.0%)     1 (0.1%, 19.6%)  <u64>::from_ascii_radix
    271 (0.5%, 50.5%)     1 (0.1%, 19.7%)  <usize>::from_ascii_radix
    260 (0.5%, 51.0%)     5 (0.3%, 20.0%)  <core::iter::adapters::enumerate::Enumerate<I> as core::iter::traits::iterator::Iterator>::next
    256 (0.5%, 51.5%)     4 (0.2%, 20.2%)  <core::ops::index_range::IndexRange as core::iter::traits::iterator::Iterator>::try_fold
    252 (0.5%, 52.0%)     3 (0.2%, 20.4%)  <alloc::collections::btree::map::BTreeMap<K,V,A> as core::ops::drop::Drop>::drop
    247 (0.5%, 52.4%)     1 (0.1%, 20.5%)  melee_sim::parse_bone_scale
```

</details>

<details><summary>Before: sim-lib</summary>

```text
  Lines                 Copies               Function name
  -----                 ------               -------------
  764422                12604                (TOTAL)
   25921 (3.4%,  3.4%)     49 (0.4%,  0.4%)  melee_ft::fighter::grab_throw::release_back_throw
   21511 (2.8%,  6.2%)     49 (0.4%,  0.8%)  melee_ft::fighter::damage::detect_hit
   20090 (2.6%,  8.8%)     49 (0.4%,  1.2%)  melee_ft::fighter::grab_throw::enter_back_throw
   12103 (1.6%, 10.4%)     49 (0.4%,  1.6%)  melee_ft::fighter::grab::capture_pair
    9653 (1.3%, 11.7%)     49 (0.4%,  1.9%)  melee_ft::fighter::damage::record_shield_hit
    9347 (1.2%, 12.9%)    136 (1.1%,  3.0%)  core::iter::traits::iterator::Iterator::try_fold
    9272 (1.2%, 14.1%)    112 (0.9%,  3.9%)  <alloc::vec::Vec<T> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<T,I>>::from_iter
    8897 (1.2%, 15.3%)      7 (0.1%,  4.0%)  melee_sim::effects::Effects::flush
    8745 (1.1%, 16.4%)     80 (0.6%,  4.6%)  <core::slice::iter::Iter<T> as core::iter::traits::iterator::Iterator>::fold
    8526 (1.1%, 17.5%)     49 (0.4%,  5.0%)  melee_ft::fighter::grab::candidate
    8350 (1.1%, 18.6%)     23 (0.2%,  5.2%)  <toml::de::deserializer::value::ValueDeserializer as serde_core::de::Deserializer>::deserialize_any
    7463 (1.0%, 19.6%)     99 (0.8%,  6.0%)  alloc::vec::Vec<T,A>::extend_trusted
    7375 (1.0%, 20.6%)      7 (0.1%,  6.0%)  melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::prepare
    6685 (0.9%, 21.4%)      7 (0.1%,  6.1%)  melee_sim::initial_state::fighter::import
    5978 (0.8%, 22.2%)     98 (0.8%,  6.8%)  core::iter::adapters::flatten::try_flatten_one::{{closure}}
    5745 (0.8%, 23.0%)     99 (0.8%,  7.6%)  <core::iter::adapters::fuse::Fuse<I> as core::iter::adapters::fuse::FuseImpl<I>>::try_fold
    5656 (0.7%, 23.7%)      7 (0.1%,  7.7%)  melee_lb::dynamics::DynamicBoneSet::solve
    5243 (0.7%, 24.4%)     49 (0.4%,  8.1%)  melee_ft::fighter::grab_throw::update_constraint
    5117 (0.7%, 25.1%)      7 (0.1%,  8.1%)  melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::change_motion_state_with_options
    5059 (0.7%, 25.7%)     45 (0.4%,  8.5%)  alloc::collections::btree::node::NodeRef<BorrowType,K,V,Type>::ascend
    4949 (0.6%, 26.4%)     45 (0.4%,  8.8%)  <core::slice::iter::Iter<T> as core::iter::traits::iterator::Iterator>::position
    4935 (0.6%, 27.0%)      3 (0.0%,  8.9%)  <melee_sim::scenario::_::<impl serde_core::de::Deserialize for melee_sim::scenario::Scenario>::deserialize::__Visitor as serde_core::de::Visitor>::visit_map
    4708 (0.6%, 27.6%)      1 (0.0%,  8.9%)  melee_sim::frame::Runtime::dispatch
    4448 (0.6%, 28.2%)     32 (0.3%,  9.1%)  alloc::collections::btree::navigate::<impl alloc::collections::btree::node::Handle<alloc::collections::btree::node::NodeRef<BorrowType,K,V,alloc::collections::btree::node::marker::LeafOrInternal>,alloc::collections::btree::node::marker::KV>>::next_leaf_edge
    4374 (0.6%, 28.8%)     27 (0.2%,  9.3%)  alloc::collections::btree::search::<impl alloc::collections::btree::node::NodeRef<BorrowType,K,V,alloc::collections::btree::node::marker::LeafOrInternal>>::search_tree
    4116 (0.5%, 29.3%)     49 (0.4%,  9.7%)  melee_ft::fighter::grab::capture_delta
    3784 (0.5%, 29.8%)     22 (0.2%,  9.9%)  <toml::de::deserializer::table::TableMapAccess as serde_core::de::MapAccess>::next_value_seed
    3778 (0.5%, 30.3%)     35 (0.3%, 10.2%)  core::array::try_from_fn_erased
    3636 (0.5%, 30.8%)     23 (0.2%, 10.4%)  serde_core::de::Visitor::visit_i128
    3636 (0.5%, 31.3%)     23 (0.2%, 10.6%)  serde_core::de::Visitor::visit_u128
    3577 (0.5%, 31.7%)      7 (0.1%, 10.6%)  melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::create
    3563 (0.5%, 32.2%)      1 (0.0%, 10.6%)  melee_sim::initial_state::InitialState::from_savestate_traces
    3498 (0.5%, 32.7%)      3 (0.0%, 10.6%)  <slp::cold::_::<impl serde_core::de::Deserialize for slp::cold::ControllerFrame>::deserialize::__Visitor as serde_core::de::Visitor>::visit_map
    3346 (0.4%, 33.1%)      7 (0.1%, 10.7%)  melee_sim::initial_state::saved_pose::SavedPose::restore
    3269 (0.4%, 33.5%)      6 (0.0%, 10.7%)  <&mut serde_json::de::Deserializer<R> as serde_core::de::Deserializer>::deserialize_any
    3200 (0.4%, 34.0%)     12 (0.1%, 10.8%)  alloc::collections::btree::node::Handle<alloc::collections::btree::node::NodeRef<alloc::collections::btree::node::marker::Mut,K,V,alloc::collections::btree::node::marker::Leaf>,alloc::collections::btree::node::marker::Edge>::insert_recursing
    3131 (0.4%, 34.4%)    111 (0.9%, 11.7%)  core::iter::traits::iterator::Iterator::find::check::{{closure}}
    3103 (0.4%, 34.8%)     29 (0.2%, 11.9%)  alloc::collections::btree::navigate::LazyLeafRange<BorrowType,K,V>::init_front
    3075 (0.4%, 35.2%)      3 (0.0%, 12.0%)  <slp::cold::_::<impl serde_core::de::Deserialize for slp::cold::ReplayRules>::deserialize::__Visitor as serde_core::de::Visitor>::visit_map
    3024 (0.4%, 35.6%)      7 (0.1%, 12.0%)  melee_ft::fighter::damage::<impl melee_ft::fighter::Fighter<C>>::begin_damage_reaction
    3022 (0.4%, 36.0%)     98 (0.8%, 12.8%)  <alloc::boxed::Box<T,A> as core::ops::drop::Drop>::drop
    2878 (0.4%, 36.3%)     98 (0.8%, 13.6%)  core::iter::adapters::map::map_fold::{{closure}}
    2856 (0.4%, 36.7%)      7 (0.1%, 13.6%)  melee_ft::fighter::effects::<impl melee_ft::fighter::Fighter<C>>::resolve_graphics_commands
    2737 (0.4%, 37.1%)     23 (0.2%, 13.8%)  alloc::collections::btree::navigate::<impl alloc::collections::btree::node::Handle<alloc::collections::btree::node::NodeRef<alloc::collections::btree::node::marker::Dying,K,V,alloc::collections::btree::node::marker::Leaf>,alloc::collections::btree::node::marker::Edge>>::deallocating_next
    2674 (0.3%, 37.4%)      7 (0.1%, 13.9%)  melee_ft::fighter::effects::<impl melee_ft::fighter::Fighter<C>>::flush_effects_on_motion_change
    2597 (0.3%, 37.8%)     24 (0.2%, 14.1%)  <toml::de::deserializer::array::ArraySeqAccess as serde_core::de::SeqAccess>::next_element_seed
    2562 (0.3%, 38.1%)      7 (0.1%, 14.1%)  melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield
    2552 (0.3%, 38.4%)     22 (0.2%, 14.3%)  <toml::de::deserializer::table::TableMapAccess as serde_core::de::MapAccess>::next_value_seed::{{closure}}
    2548 (0.3%, 38.8%)      7 (0.1%, 14.4%)  melee_sim::frame::dispatch_fighter
    2541 (0.3%, 39.1%)     27 (0.2%, 14.6%)  alloc::collections::btree::search::<impl alloc::collections::btree::node::NodeRef<BorrowType,K,V,Type>>::find_key_index
    2500 (0.3%, 39.4%)     20 (0.2%, 14.7%)  <serde_spanned::de::SpannedDeserializer<T,E> as serde_core::de::MapAccess>::next_value_seed
    2466 (0.3%, 39.7%)      1 (0.0%, 14.7%)  melee_sim::frame::grab_pairs::align
    2450 (0.3%, 40.1%)     24 (0.2%, 14.9%)  alloc::collections::btree::node::Handle<alloc::collections::btree::node::NodeRef<alloc::collections::btree::node::marker::Mut,K,V,NodeType>,alloc::collections::btree::node::marker::KV>::split_leaf_data
    2422 (0.3%, 40.4%)      7 (0.1%, 15.0%)  melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input
    2415 (0.3%, 40.7%)     23 (0.2%, 15.2%)  alloc::collections::btree::navigate::LazyLeafRange<alloc::collections::btree::node::marker::Dying,K,V>::take_front
    2385 (0.3%, 41.0%)      3 (0.0%, 15.2%)  <melee_sim::scenario::_::<impl serde_core::de::Deserialize for melee_sim::scenario::FighterScenario>::deserialize::__Visitor as serde_core::de::Visitor>::visit_map
    2362 (0.3%, 41.3%)     98 (0.8%, 16.0%)  core::ops::function::FnOnce::call_once
```

</details>

<details><summary>After: sim-lib</summary>

```text
  Lines                 Copies              Function name
  -----                 ------              -------------
  535335                9842                (TOTAL)
    8897 (1.7%,  1.7%)     7 (0.1%,  0.1%)  melee_sim::effects::Effects::flush
    8673 (1.6%,  3.3%)    49 (0.5%,  0.6%)  melee_ft::fighter::grab::capture_pair
    8350 (1.6%,  4.8%)    23 (0.2%,  0.8%)  <toml::de::deserializer::value::ValueDeserializer as serde_core::de::Deserializer>::deserialize_any
    8036 (1.5%,  6.3%)    49 (0.5%,  1.3%)  melee_ft::fighter::grab_throw::enter_back_throw
    6783 (1.3%,  7.6%)     7 (0.1%,  1.4%)  melee_sim::initial_state::fighter::import
    5059 (0.9%,  8.6%)    45 (0.5%,  1.8%)  alloc::collections::btree::node::NodeRef<BorrowType,K,V,Type>::ascend
    4935 (0.9%,  9.5%)     3 (0.0%,  1.9%)  <melee_sim::scenario::_::<impl serde_core::de::Deserialize for melee_sim::scenario::Scenario>::deserialize::__Visitor as serde_core::de::Visitor>::visit_map
    4851 (0.9%, 10.4%)    49 (0.5%,  2.4%)  melee_ft::fighter::damage::detect_hit
    4673 (0.9%, 11.3%)     1 (0.0%,  2.4%)  melee_sim::frame::Runtime::dispatch
    4505 (0.8%, 12.1%)    49 (0.5%,  2.9%)  <alloc::vec::Vec<T> as alloc::vec::spec_from_iter_nested::SpecFromIterNested<T,I>>::from_iter
    4448 (0.8%, 12.9%)    32 (0.3%,  3.2%)  alloc::collections::btree::navigate::<impl alloc::collections::btree::node::Handle<alloc::collections::btree::node::NodeRef<BorrowType,K,V,alloc::collections::btree::node::marker::LeafOrInternal>,alloc::collections::btree::node::marker::KV>>::next_leaf_edge
    3888 (0.7%, 13.7%)    24 (0.2%,  3.4%)  alloc::collections::btree::search::<impl alloc::collections::btree::node::NodeRef<BorrowType,K,V,alloc::collections::btree::node::marker::LeafOrInternal>>::search_tree
    3784 (0.7%, 14.4%)    22 (0.2%,  3.7%)  <toml::de::deserializer::table::TableMapAccess as serde_core::de::MapAccess>::next_value_seed
    3636 (0.7%, 15.0%)    23 (0.2%,  3.9%)  serde_core::de::Visitor::visit_i128
    3636 (0.7%, 15.7%)    23 (0.2%,  4.1%)  serde_core::de::Visitor::visit_u128
    3630 (0.7%, 16.4%)     7 (0.1%,  4.2%)  melee_ft::fighter::spawn::<impl melee_ft::fighter::Fighter<C>>::create
    3563 (0.7%, 17.1%)     1 (0.0%,  4.2%)  melee_sim::initial_state::InitialState::from_savestate_traces
    3498 (0.7%, 17.7%)     3 (0.0%,  4.2%)  <slp::cold::_::<impl serde_core::de::Deserialize for slp::cold::ControllerFrame>::deserialize::__Visitor as serde_core::de::Visitor>::visit_map
    3409 (0.6%, 18.4%)    31 (0.3%,  4.6%)  <core::slice::iter::Iter<T> as core::iter::traits::iterator::Iterator>::position
    3269 (0.6%, 19.0%)     6 (0.1%,  4.6%)  <&mut serde_json::de::Deserializer<R> as serde_core::de::Deserializer>::deserialize_any
    3200 (0.6%, 19.6%)    12 (0.1%,  4.7%)  alloc::collections::btree::node::Handle<alloc::collections::btree::node::NodeRef<alloc::collections::btree::node::marker::Mut,K,V,alloc::collections::btree::node::marker::Leaf>,alloc::collections::btree::node::marker::Edge>::insert_recursing
    3103 (0.6%, 20.1%)    29 (0.3%,  5.0%)  alloc::collections::btree::navigate::LazyLeafRange<BorrowType,K,V>::init_front
    3075 (0.6%, 20.7%)     3 (0.0%,  5.1%)  <slp::cold::_::<impl serde_core::de::Deserialize for slp::cold::ReplayRules>::deserialize::__Visitor as serde_core::de::Visitor>::visit_map
    3022 (0.6%, 21.3%)    98 (1.0%,  6.1%)  <alloc::boxed::Box<T,A> as core::ops::drop::Drop>::drop
    2848 (0.5%, 21.8%)    27 (0.3%,  6.3%)  core::array::try_from_fn_erased
    2737 (0.5%, 22.3%)    23 (0.2%,  6.6%)  alloc::collections::btree::navigate::<impl alloc::collections::btree::node::Handle<alloc::collections::btree::node::NodeRef<alloc::collections::btree::node::marker::Dying,K,V,alloc::collections::btree::node::marker::Leaf>,alloc::collections::btree::node::marker::Edge>>::deallocating_next
    2696 (0.5%, 22.8%)    36 (0.4%,  6.9%)  alloc::vec::Vec<T,A>::extend_trusted
    2597 (0.5%, 23.3%)    24 (0.2%,  7.2%)  <toml::de::deserializer::array::ArraySeqAccess as serde_core::de::SeqAccess>::next_element_seed
    2552 (0.5%, 23.8%)    22 (0.2%,  7.4%)  <toml::de::deserializer::table::TableMapAccess as serde_core::de::MapAccess>::next_value_seed::{{closure}}
    2548 (0.5%, 24.3%)    49 (0.5%,  7.9%)  melee_ft::fighter::grab_throw::release_back_throw
    2500 (0.5%, 24.7%)    20 (0.2%,  8.1%)  <serde_spanned::de::SpannedDeserializer<T,E> as serde_core::de::MapAccess>::next_value_seed
    2450 (0.5%, 25.2%)    24 (0.2%,  8.3%)  alloc::collections::btree::node::Handle<alloc::collections::btree::node::NodeRef<alloc::collections::btree::node::marker::Mut,K,V,NodeType>,alloc::collections::btree::node::marker::KV>::split_leaf_data
    2450 (0.5%, 25.6%)     7 (0.1%,  8.4%)  melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::enter_shield
    2422 (0.5%, 26.1%)     7 (0.1%,  8.5%)  melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_input
    2415 (0.5%, 26.6%)    23 (0.2%,  8.7%)  alloc::collections::btree::navigate::LazyLeafRange<alloc::collections::btree::node::marker::Dying,K,V>::take_front
    2385 (0.4%, 27.0%)     3 (0.0%,  8.7%)  <melee_sim::scenario::_::<impl serde_core::de::Deserialize for melee_sim::scenario::FighterScenario>::deserialize::__Visitor as serde_core::de::Visitor>::visit_map
    2331 (0.4%, 27.4%)    12 (0.1%,  8.9%)  alloc::collections::btree::node::Handle<alloc::collections::btree::node::NodeRef<alloc::collections::btree::node::marker::Mut,K,V,alloc::collections::btree::node::marker::Leaf>,alloc::collections::btree::node::marker::Edge>::insert
    2311 (0.4%, 27.9%)     1 (0.0%,  8.9%)  melee_sim::replay::run
    2310 (0.4%, 28.3%)     7 (0.1%,  9.0%)  melee_ft::fighter::shield::<impl melee_ft::fighter::Fighter<C>>::shield_input
    2295 (0.4%, 28.7%)     1 (0.0%,  9.0%)  melee_sim::frame::grab_pairs::select
    2259 (0.4%, 29.1%)    24 (0.2%,  9.2%)  alloc::collections::btree::search::<impl alloc::collections::btree::node::NodeRef<BorrowType,K,V,Type>>::find_key_index
    2240 (0.4%, 29.6%)     7 (0.1%,  9.3%)  melee_sim::frame::dispatch_fighter
    2191 (0.4%, 30.0%)     7 (0.1%,  9.3%)  melee_ft::fighter::turn::<impl melee_ft::fighter::Fighter<C>>::turn_input
    2142 (0.4%, 30.4%)     7 (0.1%,  9.4%)  melee_ft::fighter::walk::<impl melee_ft::fighter::Fighter<C>>::walk_input
    2114 (0.4%, 30.8%)     7 (0.1%,  9.5%)  melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_animation
    2107 (0.4%, 31.2%)     7 (0.1%,  9.6%)  melee_ft::fighter::squat::<impl melee_ft::fighter::Fighter<C>>::squat_input
    2001 (0.4%, 31.5%)    23 (0.2%,  9.8%)  <toml::de::deserializer::table::TableDeserializer as serde_core::de::Deserializer>::deserialize_any
    1988 (0.4%, 31.9%)    28 (0.3%, 10.1%)  alloc::boxed::Box<T,A>::try_new_uninit_in
    1947 (0.4%, 32.3%)    12 (0.1%, 10.2%)  alloc::collections::btree::node::Handle<alloc::collections::btree::node::NodeRef<alloc::collections::btree::node::marker::Mut,K,V,alloc::collections::btree::node::marker::Internal>,alloc::collections::btree::node::marker::Edge>::insert
    1932 (0.4%, 32.6%)    23 (0.2%, 10.4%)  <alloc::collections::btree::map::BTreeMap<K,V,A> as core::ops::drop::Drop>::drop
    1878 (0.4%, 33.0%)    17 (0.2%, 10.6%)  <core::slice::iter::Iter<T> as core::iter::traits::iterator::Iterator>::fold
    1867 (0.3%, 33.3%)     2 (0.0%, 10.6%)  <melee_diff::_::<impl serde_core::de::Deserialize for melee_diff::Value>::deserialize::__Visitor as serde_core::de::Visitor>::visit_map
    1850 (0.3%, 33.7%)    12 (0.1%, 10.7%)  alloc::collections::btree::map::entry::VacantEntry<K,V,A>::insert_entry
    1848 (0.3%, 34.0%)     6 (0.1%, 10.8%)  <<melee_diff::_::<impl serde_core::de::Deserialize for melee_diff::Value>::deserialize::__Seed as serde_core::de::DeserializeSeed>::deserialize::__Visitor as serde_core::de::Visitor>::visit_map
    1843 (0.3%, 34.4%)     7 (0.1%, 10.9%)  melee_ft::fighter::life::<impl melee_ft::fighter::Fighter<C>>::reset_for_revival
    1820 (0.3%, 34.7%)    19 (0.2%, 11.1%)  alloc::vec::Vec<T,A>::extend_desugared
    1820 (0.3%, 35.0%)     7 (0.1%, 11.1%)  melee_ft::fighter::ledge::<impl melee_ft::fighter::Fighter<C>>::ledge_collision
```

</details>
