# C1 motion state tables

Final review revision, 2026-09-09. Callback dispatch now uses explicit borrowed
phase resources and ordinary return values. The rejected `state/context.rs`, its
three tests, and all fighter TLS/unsafe code are deleted. Full oracle acceptance
remains blocked by missing local assets.

## Structure and scope

- `state/common_table.rs` replaces 47 `MotionState` constants and the additional
  motion-entry match arms with **64 implemented common rows and 277
  `unimplemented_row` entries**, covering all 341 retail common indices.
- `state/row.rs` defines copyable `MotionRow<C>` and live
  `MotionState<C> { action: ActionId, row: MotionRow<C> }`. Read-only dereferencing
  exposes the row's existing semantic `id`, preserving the existing state readers
  and raw comparators without duplicating that metadata.
- `Fighter::row` resolves the existing `CharacterCallbacks::action_id()` override
  through table lookup. Motion entry takes `ActionId` or a common enum convertible
  into it, installs the selected row, and retains the old entry side effects.
  Its large internal body has a fixed `ActionId` parameter, keeping the conversion
  wrappers thin.
- `state/callbacks/` holds the extracted scheduler-arm bodies and thin wrappers
  around the existing state-module functions. Animation, IASA, physics, collision
  and camera each invoke the **current** row in their original scheduler phase.
  State changes in an earlier phase therefore affect later phases immediately.
- The existing Puff rows 341–345 live in `ft-purin/src/init.rs`, in retail
  `ftPr_Init_MotionStateTable` order. They retain shared multijump animation and
  physics, aerial IASA, Fall collision without StopCeil, and animations 295–299.
- The existing Yoshi shield rows 341–345 live in `ft-yoshi/src/init.rs`, in retail
  `ftYs_Init_MotionStateTable` order. The common semantic identities and existing
  shield hooks remain, while the live action numbers come from those rows.
- Every character retains the default no-op `enter_special`. The shared grounded
  and aerial input-transition paths now call the hook for a recognized special
  buffer. These entry bodies remain stubs; no existing gated scene reaches them
  (the corresponding transition previously panicked). Specials remain S3.

No enum-hidden behavior difference was found or intentionally corrected.
Operation order is preserved, including death's skipped animation step, idle
restart/RNG work, float checks around aerial jumps, ledge-before-ceiling checks,
landing RNG after collision, and camera-box update before ledge notification.
The complete bit-exact claim still needs the missing-asset rerun described below.

The five callback enums and `StateCallbacks` are gone. The required literal grep
also matched a pre-existing stage type, `JointCollisionCallback`, and occurrences
inside the retail camera symbol. To satisfy that exact scan, the stage alias is
now `JointCollisionHandler` (same fn type), and camera comments cite its retail
address. Those are naming/documentation-only changes in `melee-mp`, `hsd-gobj`,
and the fighter README.

`scene_characters!`, `SceneFighter`, M4/M5 gate definitions, scenario definitions,
comparator logic, expected values, effects implementation and decomp are
unchanged. Test edits only migrate state construction/access and callback identity
assertions to the new API. The existing `wait_callback_table` test is retained.
The three rejected context-scope tests were deleted with their implementation;
no ignore annotation was changed.
C2/C7 and code movement between crates were not started.

## Row layout and retail field mapping

Retail references: `ft/types.h:853-884`, `ftmotionstates.c`'s
`ftData_MotionStateList`, `ftdata.c`'s `ftData_CharacterStateTables`, and
`Fighter_ChangeMotionState` (`fighter.c:1177-1181`).

| Rust field | Mapping |
|---|---|
| `animation: i32` | Retail `anim_id`; retains each existing `spawn.rs` submotion value, including -1 for no animation. |
| `anim`, `iasa`, `physics`, `collision`, `camera` | Five phase-specific fn pointers (signatures below), corresponding to retail `anim_cb`, `input_cb`, `phys_cb`, `coll_cb`, `cam_cb`. |
| `action: ActionId` | Existing action number, recorded in the row so common-to-character remapping installs the correct live ID. |
| `id: CommonMotionState` | Existing shared semantic identity used by state scratch and character hooks; distinct from a character row's live action number. |
| `implemented: bool` | Existing port-coverage distinction, formerly the supported motion-entry match arms. |
| Retail `x4_flags` / packed move ID | Not yet modelled by the current port; omitted with a source comment. Playback `MotionFlags` belongs to a different word and is not substituted. |

`ActionId(u16)` uses `COMMON_COUNT = 341`; character rows are indexed after
subtracting 341. Unported common rows install panic functions for all five
callbacks. Their diagnostic retains the old unsupported-motion-entry marker and
adds the exact `ftCo_MS_*` name and retail table index. The full name list is
constant data in `state/names.rs`.

## Explicit phase resources and results

`state/phase.rs` defines the existing scheduler inputs as ordinary structs.
Each callback receives its phase by value; borrowed resources remain owned by the
scheduler's caller. No context stores output or uses hidden global state.

| Phase | Fields (exact types) | Callback return |
|---|---|---|
| `AnimationPhase<'a>` | `assets: &'a FighterAssets`, `rng: &'a mut HsdRng` | `Result<Option<WaitChoice>>` |
| `InputPhase<'a>` | `assets: &'a FighterAssets` | `()` |
| `PhysicsPhase<'a>` | `assets: &'a FighterAssets`, `map: &'a CollMap`, `wind: Vec3` | `()` |
| `CollisionPhase<'a>` | `assets: Option<&'a FighterAssets>`, `map: &'a mut CollMap` | `Result<()>` |
| `CameraPhase<'a>` | `assets: &'a FighterAssets`, `zoom: f32` | `()` |

```rust
pub type AnimFn<C> = fn(&mut Fighter<C>, AnimationPhase<'_>) -> Result<Option<WaitChoice>>;
pub type InputFn<C> = fn(&mut Fighter<C>, InputPhase<'_>);
pub type PhysicsFn<C> = fn(&mut Fighter<C>, PhysicsPhase<'_>);
pub type CollisionFn<C> = fn(&mut Fighter<C>, CollisionPhase<'_>) -> Result<()>;
pub type CameraFn<C> = fn(&mut Fighter<C>, CameraPhase<'_>);
```

`procs.rs` constructs these phases at each existing dispatch site. The callback
modules unpack them directly, or ignore `_phase` when no resource is needed.
Animation playback and smash-charge advancement previously performed by the
adapter now appear explicitly before each corresponding animation body; death
continues to skip that preamble. Collision retains the optional assets needed by
the existing map-only API, while landing-effect RNG remains in the scheduler
epilogue. Animation and collision errors return directly to their callers.

`MotionRow` remains `Copy`, with the same metadata and five pointers. The common
table, names table, special hook, Puff table and Yoshi table are byte-identical to
the pre-review snapshot. Callback identity tests were adapted to the distinct fn
types; their expected callbacks and state values are unchanged. A normalized
source comparison verified all 96 migrated callback bodies against the snapshot,
including preservation of the animation preamble's operation order. No hidden
behavior difference was identified; this inspection does not replace oracle tests.

This review revision changes `procs.rs`, `state.rs`, `state/row.rs`, the five
`state/callbacks/*.rs` implementations, `tests/fox_spawn_native.rs`, and this report;
it adds `state/phase.rs` and deletes `state/context.rs`. The complete C1 file list
is below. No C2/C7 work was started.

## Validation limits

Both shared local asset directories, `harness/roms` and `harness/traces`, contain
zero entries. They were inspected read-only and left untouched.

**The m4/m5 gate tests return early and pass when their local traces are absent.**
That behavior was not changed. Their green results below therefore do not prove
bit-exact oracle equivalence. Asset-dependent fighter and particle live/dust
replays also could not execute their replay checks. The full oracle rerun remains
blocked until the local assets are restored; all requested check commands were run.
No test, comparator, expected value, scenario, trace or ignore annotation was
changed to bypass this limitation.

The pre-review `cargo gate` run reported 944 passed, 0 failed, 3 ignored
(`/private/tmp/c1-phase-baseline.log`). The final total is 941 passed, 0 failed,
3 ignored: the only test-count reduction is deletion of the three context-scope
tests requested in review. The m5 ignored count remains zero.

## Exact final commands and results

Totals aggregate test binaries and doc-tests. All commands in this table exited
0; the existing missing-asset early returns are included in the pass counts.

| Command | Passed / failed / ignored | Final nonblank output line |
|---|---|---|
| `cargo test -p melee-sim --test m4_gate` | 261 / 0 / 0 | `test result: ok. 261 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s` |
| `cargo test -p melee-sim --test m5_gate` | 8 / 0 / 0 | `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` |
| `cargo test -p melee-ft` | 90 / 0 / 0 | `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` |
| `cargo test -p hsd-particle` | 75 / 0 / 0 | `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` |
| `cargo gate` | 941 / 0 / 3 | `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s` |
| `cargo clippy --workspace --all-targets -- -D warnings` | n/a | ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 15.23s`` |
| `cargo fmt --all` | n/a | No output |
| `cargo fmt --all -- --check` | n/a | No output |

```sh
grep -rn 'unsafe\|thread_local' crates/melee-ft/src/fighter/
grep -rn 'AnimationCallback\|InputCallback\|PhysicsCallback\|CollisionCallback\|CameraCallback' crates/
```

Both scans produced no output and exited 1 (no matches): both absence checks pass.

Logs: `/private/tmp/c1-phase-{m4,m5,ft,particle,gate,clippy,fmt,fmt-check,no-context,no-enums}.log`.
Command/exit manifest: `/private/tmp/c1-phase-validation.json`.

## Files changed

- `crates/ft-purin/src/init.rs`
- `crates/ft-yoshi/src/init.rs`
- `crates/hsd-gobj/src/consts.rs`
- `crates/melee-ft/src/fighter/README.md`
- `crates/melee-ft/src/fighter/mod.rs`
- `crates/melee-ft/src/fighter/multi_jump.rs`
- `crates/melee-ft/src/fighter/procs.rs`
- `crates/melee-ft/src/fighter/snapshot.rs`
- `crates/melee-ft/src/fighter/spawn.rs`
- `crates/melee-ft/src/fighter/state.rs`
- `crates/melee-ft/src/fighter/state/action.rs`
- `crates/melee-ft/src/fighter/state/callbacks/animation.rs`
- `crates/melee-ft/src/fighter/state/callbacks/camera.rs`
- `crates/melee-ft/src/fighter/state/callbacks/collision.rs`
- `crates/melee-ft/src/fighter/state/callbacks/input.rs`
- `crates/melee-ft/src/fighter/state/callbacks/mod.rs`
- `crates/melee-ft/src/fighter/state/callbacks/physics.rs`
- `crates/melee-ft/src/fighter/state/common_table.rs`
- `crates/melee-ft/src/fighter/state/phase.rs`
- `crates/melee-ft/src/fighter/state/names.rs`
- `crates/melee-ft/src/fighter/state/row.rs`
- `crates/melee-ft/src/fighter/state/special.rs`
- `crates/melee-ft/src/fighter/walk.rs`
- `crates/melee-ft/tests/fighter_support/mod.rs`
- `crates/melee-ft/tests/fox_spawn_native.rs`
- `crates/melee-mp/src/lib.rs`
- `crates/melee-mp/src/map.rs`
- `crates/melee-sim/src/frame/puff_state.rs`
- `crates/melee-sim/src/initial_state/fighter.rs`
- `crates/melee-sim/src/replay.rs`
- `docs/PORT_NOTES/C1_STATE_TABLES.md`

`state/context.rs` was an intermediate C1 addition and is now deleted. No Git
commands or commits were made. `TRACKER.md` was not edited. The top-level scene
dispatch, gate definitions, harness assets and decomp were left untouched.
